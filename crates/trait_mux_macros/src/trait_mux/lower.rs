use convert_case::{Case, Casing};

use proc_macro2::{Ident, Span};
use syn::Path;

use super::analyze::{self, Model, Trait};

pub struct TraitAggregate<'t> {
    pub name: &'t Ident,
    pub traits: Vec<&'t Trait<'t>>,
}

pub struct EnumVariant<'t> {
    /// The enum ident, used both for the enum variant name, and (if needed) for the trait aggregate
    /// name.
    /// e.g. `BinaryDebugDisplay`
    pub ident: &'t Ident,
    pub constraint: Constraint<'t>,
}

pub struct Enum<'t> {
    pub name: &'t Ident,
    pub variants: Vec<EnumVariant<'t>>,
}

/// Represents a function derived from a trait, including its identifier,
/// result path, and the variants it applies to.
pub struct Function<'t> {
    pub name: Ident,
    pub result_path: &'t Path,
    pub matching_variants: Vec<&'t Ident>,
}

pub struct EnumImpl<'t> {
    pub functions: Vec<Function<'t>>,
}

pub enum Constraint<'t> {
    None,
    Path(&'t Path),
    Ident(&'t Ident),
}

pub struct AutorefSpecializer<'t> {
    pub tag: Ident,
    pub r#match: Ident,
    pub deref_count: usize,
    pub variant: &'t Ident,
    pub constraint: Constraint<'t>,
}

/// Intermediate representation (IR) of the parsed AST.
/// Contains the identifier, enum variants, and functions derived from the AST.
pub struct Ir<'t> {
    pub trait_aggregates: Vec<TraitAggregate<'t>>,
    pub r#enum: Enum<'t>,
    pub enum_impl: EnumImpl<'t>,
    pub autoref_specializers: Vec<AutorefSpecializer<'t>>,
    pub wrap_ident: &'t Ident,
    pub wrap_derefs: usize,
    pub into: Ident,
    pub into_tag: Ident,
}

/// Converts the given AST into its intermediate representation (IR).
/// This includes extracting traits, generating enum variants, and creating functions.
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

fn enum_variant_to_constraint<'t>(v: &'t analyze::EnumVariant<'t>) -> Constraint<'t> {
    match v.implemented_traits.len() {
        0 => Constraint::None,
        1 => Constraint::Path(v.implemented_traits[0].path),
        _ => Constraint::Ident(&v.ident),
    }
}

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
/// that implement the trait.
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
