use chronlyt_plugin_validator::validate_component_contract;

#[test]
fn frozen_component_contract_cases() {
    for (source, expected) in [
        (
            include_str!("../../../fixtures/v1/component/valid.wat"),
            true,
        ),
        (
            include_str!("../../../fixtures/v1/component/extra-export.wat"),
            true,
        ),
        (
            include_str!("../../../fixtures/v1/component/host-subset.wat"),
            true,
        ),
        (
            include_str!("../../../fixtures/v1/component/start-trap.wat"),
            true,
        ),
        (
            include_str!("../../../fixtures/v1/component/parameter-label.wat"),
            true,
        ),
        (
            include_str!("../../../fixtures/v1/component/wrong-export.wat"),
            false,
        ),
        (
            include_str!("../../../fixtures/v1/component/wrong-export-type.wat"),
            false,
        ),
        (
            include_str!("../../../fixtures/v1/component/wrong-host-version.wat"),
            false,
        ),
        (
            include_str!("../../../fixtures/v1/component/wrong-host-type.wat"),
            false,
        ),
        (
            include_str!("../../../fixtures/v1/component/wasi-import.wat"),
            false,
        ),
    ] {
        let bytes = wat::parse_str(source).unwrap();
        // Negative ABI fixtures must still be well-formed components, so a
        // decoder failure cannot mask a missing signature/import restriction.
        wasmparser::Validator::new().validate_all(&bytes).unwrap();
        assert_eq!(
            validate_component_contract(&bytes).is_ok(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn invalid_empty_core_module_and_oversized_inputs_fail() {
    for bytes in [
        b"".as_slice(),
        b"not wasm",
        b"\0asm\x0d\0\x01\0",
        b"\0asm\x01\0\0\0",
    ] {
        assert!(validate_component_contract(bytes).is_err());
    }
    assert!(validate_component_contract(&vec![0; 32 * 1024 * 1024 + 1]).is_err());
}
