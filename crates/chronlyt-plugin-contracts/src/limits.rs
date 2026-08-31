//! Wire bounds only. Fuel, deadlines, Store resources and transactional quotas
//! are host policy and deliberately absent here.

macro_rules! wire_limits {
    ($($name:ident: $ty:ty = $value:expr;)*) => {
        $(pub const $name: $ty = $value;)*
        #[cfg(feature = "schema")]
        pub const WIRE_LIMITS: &[(&str, u64)] = &[
            $((stringify!($name), $name as u64),)*
        ];
    };
}

wire_limits! {
    MAX_MANIFEST_BYTES: usize = 128 * 1024;
    MAX_PLUGIN_ID_BYTES: usize = 57;
    MAX_NAME_BYTES: usize = 80;
    MAX_DESCRIPTION_BYTES: usize = 2_000;
    MAX_AUTHOR_BYTES: usize = 120;
    MAX_PAGES: usize = 32;
    MAX_ASSETS: usize = 64;
    MAX_PATH_BYTES: usize = 240;
    MAX_IDENTIFIER_BYTES: usize = 64;
    MAX_PERMISSIONS: usize = 32;
    MAX_CATALOG_BYTES: usize = 2 * 1024 * 1024;
    MAX_CATALOG_ENTRIES: usize = 1_000;
    MAX_CATALOG_SUMMARY_BYTES: usize = 300;
    MAX_CATALOG_DESCRIPTION_BYTES: usize = 4_000;
    MAX_CATALOG_LICENSE_BYTES: usize = 80;
    MAX_CATALOG_CHANGELOG_BYTES: usize = 8_000;
    MAX_CATALOG_CATEGORIES: usize = 16;
    MAX_CATALOG_TAGS: usize = 32;
    MAX_CATALOG_LABEL_BYTES: usize = 64;
    MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;
    MAX_COMPONENT_BYTES: usize = 32 * 1024 * 1024;
    MAX_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
    MAX_ASSET_BYTES: usize = 4 * 1024 * 1024;
    MAX_ARCHIVE_ENTRIES: usize = 256;
    MAX_HOST_INPUT_BYTES: usize = 64 * 1024;
    MAX_HOST_OUTPUT_BYTES: usize = 256 * 1024;
    MAX_UI_NODES: usize = 512;
    MAX_UI_DEPTH: usize = 16;
    MAX_UI_CHILDREN: usize = 64;
    MAX_UI_STRING_BYTES: usize = 4 * 1024;
    MAX_UI_ACTION_ID_BYTES: usize = 128;
    MAX_UI_INPUTS: usize = 64;
    MAX_UI_LIST_ITEMS: usize = 100;
    MAX_TITLE_CHARACTERS: usize = 200;
    MAX_NOTE_CHARACTERS: usize = 4_000;
    MAX_DURATION_SECONDS: i64 = 31_536_000;
    MAX_SOURCE_REF_BYTES: usize = 200;
    MAX_STORAGE_KEY_BYTES: usize = 128;
    MAX_STORAGE_VALUE_BYTES: usize = 64 * 1024;
    MAX_TIMELINE_QUERY_RESULTS: usize = 100;
    DEFAULT_TIMELINE_QUERY_RESULTS: usize = 50;
    MAX_PAGE_DESCRIPTION_BYTES: usize = 300;
}
