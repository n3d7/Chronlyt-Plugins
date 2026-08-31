//! Static typechecking only. No engine, linker instantiation or guest execution.
use chronlyt_plugin_contracts::{ContractError, WIT_SOURCE, limits::MAX_COMPONENT_BYTES};
use wasmparser::component_types::{
    ComponentDefinedType, ComponentEntityType, ComponentFuncType, ComponentValType,
};
use wasmparser::{Parser, Payload, PrimitiveValType, Validator, types::Types};
use wit_parser::{Function, Resolve, Type, TypeDefKind, WorldItem};

use crate::{ValidationError, ValidationResult};

pub fn validate_component_contract(bytes: &[u8]) -> ValidationResult<()> {
    if bytes.len() > MAX_COMPONENT_BYTES {
        return Err(ContractError::ResourceLimit("component bytes").into());
    }
    if !Parser::is_component(bytes) {
        return Err(ValidationError::Component);
    }
    let types = Validator::new()
        .validate_all(bytes)
        .map_err(|_| ValidationError::Component)?;
    let mut resolve = Resolve::default();
    // The path is a diagnostic label. push_str does not read it from disk.
    let package = resolve
        .push_str("chronlyt-plugin.wit", WIT_SOURCE)
        .map_err(|_| ValidationError::Component)?;
    let world = resolve
        .select_world(package, Some("chronlyt-plugin"))
        .map_err(|_| ValidationError::Component)?;
    let world = &resolve.worlds[world];

    // Inspect only imports of the outer component. Nested modules/components
    // may import from internal instances and grant no ambient host authority.
    let mut depth = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|_| ValidationError::Component)? {
            Payload::Version { .. } => depth += 1,
            Payload::End(_) => depth -= 1,
            Payload::ComponentImportSection(imports) if depth == 1 => {
                for import in imports {
                    let import = import.map_err(|_| ValidationError::Component)?;
                    let expected = world
                        .imports
                        .iter()
                        .find(|(key, _)| resolve.name_world_key(key) == import.name.0)
                        .map(|(_, item)| item)
                        .ok_or(ValidationError::Component)?;
                    let WorldItem::Interface { id, .. } = expected else {
                        return Err(ValidationError::Component);
                    };
                    let Some(ComponentEntityType::Instance(instance)) =
                        types.component_entity_type_of_import(import.name.0)
                    else {
                        return Err(ValidationError::Component);
                    };
                    // The guest may require a subset of the provided functions.
                    for (name, actual) in &types[instance].exports {
                        let expected = resolve.interfaces[*id]
                            .functions
                            .get(name)
                            .ok_or(ValidationError::Component)?;
                        let ComponentEntityType::Func(actual) = actual else {
                            return Err(ValidationError::Component);
                        };
                        if !function_matches(&types, &types[*actual], &resolve, expected) {
                            return Err(ValidationError::Component);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (key, expected) in &world.exports {
        let WorldItem::Function(expected) = expected else {
            return Err(ValidationError::Component);
        };
        let Some(ComponentEntityType::Func(actual)) =
            types.component_entity_type_of_export(&resolve.name_world_key(key))
        else {
            return Err(ValidationError::Component);
        };
        if !function_matches(&types, &types[actual], &resolve, expected) {
            return Err(ValidationError::Component);
        }
    }
    // Extra exports remain permitted, as in the v1 host's typed pre-instance.
    Ok(())
}

fn function_matches(
    types: &Types,
    actual: &ComponentFuncType,
    resolve: &Resolve,
    expected: &Function,
) -> bool {
    actual.params.len() == expected.params.len()
        && actual
            .params
            .iter()
            .zip(&expected.params)
            .all(|((_, actual), (_, expected))| value_matches(types, *actual, resolve, *expected))
        && optional_matches(types, actual.result, resolve, expected.result)
}

fn optional_matches(
    types: &Types,
    actual: Option<ComponentValType>,
    resolve: &Resolve,
    expected: Option<Type>,
) -> bool {
    match (actual, expected) {
        (None, None) => true,
        (Some(actual), Some(expected)) => value_matches(types, actual, resolve, expected),
        _ => false,
    }
}

fn value_matches(
    types: &Types,
    actual: ComponentValType,
    resolve: &Resolve,
    expected: Type,
) -> bool {
    // Recurse only along the trusted WIT shape, not arbitrary guest type graphs.
    // v1 exposes strings, options and results; other shapes fail closed.
    match expected {
        Type::String => match actual {
            ComponentValType::Primitive(PrimitiveValType::String) => true,
            ComponentValType::Type(id) => matches!(
                types[id],
                ComponentDefinedType::Primitive(PrimitiveValType::String)
            ),
            _ => false,
        },
        Type::Id(id) => match &resolve.types[id].kind {
            TypeDefKind::Type(expected) => value_matches(types, actual, resolve, *expected),
            TypeDefKind::Option(expected) => match actual {
                ComponentValType::Type(id) => match &types[id] {
                    ComponentDefinedType::Option(actual) => {
                        value_matches(types, *actual, resolve, *expected)
                    }
                    _ => false,
                },
                _ => false,
            },
            TypeDefKind::Result(expected) => match actual {
                ComponentValType::Type(id) => match &types[id] {
                    ComponentDefinedType::Result { ok, err } => {
                        optional_matches(types, *ok, resolve, expected.ok)
                            && optional_matches(types, *err, resolve, expected.err)
                    }
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}
