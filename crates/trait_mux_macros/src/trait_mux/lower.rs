//! This module is responsible for converting the analyzed Model into an intermediate
//! representation (IR) that can be easily used with quote!{} macros to generate Rust code.
//! The lowering process transforms the high-level Model into structured data types that
//! closely match the output code structure.

use convert_case::{Case, Casing};

use proc_macro2::{Ident, Span};
use syn::Path;

use super::analyze::{self, Model, Trait};

/// Represents a collection of traits that need to be implemented together for a specific variant.
/// Used when a variant implements multiple traits to create trait aggregates.
pub struct TraitAggregate<'t> {
    /// The identifier for this trait aggregate, typically derived from the enum variant name.
    pub name: &'t Ident,
    /// The collection of traits that this aggregate combines.
    pub traits: Vec<&'t Trait<'t>>,
}

/// Represents a variant of the generated enum, including its identifier and trait constraints.
pub struct EnumVariant<'t> {
    /// The enum ident, used both for the enum variant name, and (if needed) for the trait aggregate
    /// name.
    /// e.g. `TypeBinaryDebugDisplay`
    pub ident: &'t Ident,
    /// The trait constraint associated with this variant, which could be None, a single trait Path,
    /// or a reference to a trait aggregate Ident.
    pub constraint: Constraint<'t>,
}

/// The main enum structure that will be generated.
pub struct Enum<'t> {
    /// The name of the enum type to be generated.
    pub name: &'t Ident,
    /// The collection of variants that will be part of this enum.
    pub variants: Vec<EnumVariant<'t>>,
}

/// Represents a function derived from a trait, including its identifier,
/// result path, and the variants it applies to.
pub struct Function<'t> {
    /// The generated function name, typically in the form `try_as_trait_name`.
    pub name: Ident,
    /// The path to the trait this function returns when successful.
    pub result_path: &'t Path,
    /// List of enum variant identifiers that can be matched by this function.
    pub matching_variants: Vec<&'t Ident>,
}

/// Contains all the functions that will be implemented for the generated enum.
pub struct EnumImpl<'t> {
    /// Collection of functions to be implemented on the enum.
    pub functions: Vec<Function<'t>>,
}

/// Specifies the kind of trait constraint applicable to an enum variant.
pub enum Constraint<'t> {
    /// No trait constraints.
    None,
    /// A constraint to a single trait path.
    Path(&'t Path),
    /// A constraint to a trait aggregate, referenced by its identifier.
    Ident(&'t Ident),
}

/// Used to generate code that uses autoref specialization to convert the user parameter into a
/// trait mux object.
pub struct AutorefSpecializer<'t> {
    /// The identifier for the tag structure associated with this specializer.
    /// e.g. `TypeBinaryDebugDisplayTag`.
    pub tag: Ident,
    /// The identifier for the match trait with this specializer.
    /// e.g. `TypeBinaryDebugDisplayMatch`.
    pub r#match: Ident,
    /// The number of dereference operations needed for this specialization.
    pub deref_count: usize,
    /// The enum variant this specializer is associated with.
    pub variant: &'t Ident,
    /// The trait constraint for this specializer.
    pub constraint: Constraint<'t>,
}

/// Intermediate representation (IR) of the parsed AST.
/// Contains all components needed to generate the final Rust code using quote!{}.
/// This structure bridges the gap between the analyzed Model and the code generation phase.
pub struct Ir<'t> {
    /// Collection of trait aggregates for variants implementing multiple traits.
    pub trait_aggregates: Vec<TraitAggregate<'t>>,
    /// The main enum that will be generated.
    pub r#enum: Enum<'t>,
    /// Contains all the functions that will be implemented for the generated enum.
    pub enum_impl: EnumImpl<'t>,
    /// Collection of autoref specializers.
    pub autoref_specializers: Vec<AutorefSpecializer<'t>>,
    /// The identifier for the wrap function.
    pub wrap_ident: &'t Ident,
    /// The number of dereference operations needed for the wrap macro.
    pub wrap_derefs: usize,
    /// The identifier for the into function.
    pub into: Ident,
    /// The identifier for the into_tag function.
    pub into_tag: Ident,
}

/// Converts the given AST Model into its intermediate representation (IR).
/// This is the main entry point for the lowering process, transforming the analyzed
/// syntax into a structure optimized for code generation.
///
/// # Arguments
/// * `model` - The analyzed Model containing traits and enum variants
///
/// # Returns
/// An Ir struct containing all components needed for code generation
pub fn lower<'t>(model: &'t Model<'t>) -> Ir<'t> {
    let trait_aggregates = generate_trait_aggregates(model);
    let r#enum = generate_enum(model);
    let enum_impl = generate_enum_impl(model);
    let autoref_specializers = generate_autoref_specializers(model);

    let into_tag = Ident::new(
        &format!(
            "into_{}_tag",
            model.enum_ident.to_string().to_case(Case::Snake)
        ),
        Span::call_site(),
    );
    let into = Ident::new(
        &format!("into_{}", model.enum_ident.to_string().to_case(Case::Snake)),
        Span::call_site(),
    );

    Ir {
        trait_aggregates,
        r#enum,
        enum_impl,
        autoref_specializers,
        wrap_ident: &model.wrap_ident,
        wrap_derefs: model.traits.len() + 1,
        into_tag,
        into,
    }
}

/// Generates trait aggregates for enum variants that implement multiple traits.
/// These aggregates will be used to create compound trait bounds for the enum variants.
///
/// # Arguments
/// * `model` - The analyzed Model containing traits and enum variants
///
/// # Returns
/// A vector of TraitAggregate structures
fn generate_trait_aggregates<'t>(model: &'t Model<'t>) -> Vec<TraitAggregate<'t>> {
    model
        .enum_variants
        .iter()
        .filter_map(|v| {
            if v.implemented_traits.len() <= 1 {
                return None;
            }

            let variant_ident = &v.ident;
            let sub_traits = v.implemented_traits.iter().collect();

            Some(TraitAggregate {
                name: variant_ident,
                traits: sub_traits,
            })
        })
        .collect()
}

/// Converts an EnumVariant from the analysis phase to a Constraint for the IR.
/// Determines the appropriate constraint type based on the number of implemented traits.
///
/// # Arguments
/// * `v` - The enum variant from the analysis phase
///
/// # Returns
/// The appropriate Constraint for the IR
fn enum_variant_to_constraint<'t>(v: &'t analyze::EnumVariant<'t>) -> Constraint<'t> {
    match v.implemented_traits.len() {
        0 => Constraint::None,
        1 => Constraint::Path(v.implemented_traits[0].path),
        _ => Constraint::Ident(&v.ident),
    }
}

/// Generates the main enum structure based on the analyzed model.
/// Creates each variant with its appropriate trait constraints.
///
/// # Arguments
/// * `model` - The analyzed Model containing traits and enum variants
///
/// # Returns
/// An Enum structure representing the main enum to be generated
fn generate_enum<'t>(model: &'t Model<'t>) -> Enum<'t> {
    let name = model.enum_ident;
    let variants = model
        .enum_variants
        .iter()
        .map(|v| {
            let constraint = enum_variant_to_constraint(v);

            EnumVariant {
                ident: &v.ident,
                constraint,
            }
        })
        .collect();

    Enum { name, variants }
}

/// Generates functions for each trait, mapping them to the enum variants
/// that implement the trait. These functions will allow accessing the underlying
/// trait implementations from the enum.
///
/// # Arguments
/// * `model` - The analyzed Model containing traits and enum variants
///
/// # Returns
/// An EnumImpl containing all functions to be implemented on the enum
fn generate_enum_impl<'t>(model: &'t Model<'t>) -> EnumImpl<'t> {
    let functions = model
        .traits
        .iter()
        .map(|current_trait| {
            let fn_name = format!(
                "try_as_{}",
                current_trait.ident.to_string().to_case(Case::Snake)
            );

            // Find all enum variants that implement the current trait.
            let matching_variants = model
                .enum_variants
                .iter()
                .filter(|v| {
                    v.implemented_traits.iter().any(|implemented_trait| {
                        core::ptr::eq(implemented_trait.path, current_trait.path)
                    })
                })
                .map(|p| &p.ident)
                .collect();

            Function {
                name: Ident::new(&fn_name, Span::call_site()),
                result_path: current_trait.path,
                matching_variants,
            }
        })
        .collect();

    EnumImpl { functions }
}

/// Generates specializers for autoref specialization.
///
/// # Arguments
/// * `model` - The analyzed Model containing traits and enum variants
///
/// # Returns
/// A vector of AutorefSpecializer structures
fn generate_autoref_specializers<'t>(model: &'t Model<'t>) -> Vec<AutorefSpecializer<'t>> {
    model
        .enum_variants
        .iter()
        .map(|v| {
            let tag = Ident::new(&format!("{}Tag", v.ident), Span::call_site());
            let r#match = Ident::new(&format!("{}Match", v.ident), Span::call_site());
            let deref_count = v.implemented_traits.len();
            let constraint = enum_variant_to_constraint(v);

            AutorefSpecializer {
                tag,
                r#match,
                deref_count,
                variant: &v.ident,
                constraint,
            }
        })
        .collect()
}
