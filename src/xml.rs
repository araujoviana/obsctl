// Creates a table-like struct for XML parsing
macro_rules! xml_table {
    ($struct_name:ident { $($renamed_field:expr => $table_field:ident : $t:ty),* $(,)? }) => {
        #[derive(tabled::Tabled)]
        pub struct $struct_name {
            $(
                #[tabled(rename = $renamed_field)]
                pub $table_field: $t,
            )*
        }
    };
}

#[macro_export]
macro_rules! generate_xml_table_vector {
    (
        $table_type:ident => $repeated_field:literal in $xml:expr, { $($xml_tag:ident => $field:ident),* $(,)? }
    ) => {{
        let doc = roxmltree::Document::parse($xml).unwrap();

        doc.descendants()
            .filter(|n| n.has_tag_name($repeated_field))
            .map(|node| {
                $(
                    let $field = node
                        .descendants()
                        .find(|n| n.has_tag_name(stringify!($xml_tag)))
                        .and_then(|n| n.text())
                        .unwrap_or("")
                        .to_string();
                )*

                $table_type {
                    $(
                        $field: $field,
                    )*
                }
            })
            .collect::<Vec<$table_type>>()
    }};
}

xml_table! {
    BucketList {
        "Name" => name: String,
        "Created At" => creation_date: String,
        "Location" => location: String,
        "Type" => bucket_type: String,
    }
}

xml_table! {
    ObjectList {
        "Key (Object Path)" => key: String,
        "Last Modified" => last_modified: String,
        // Didn't bother to add Etag but it can be easily added here
        "Size" => size: String,
        // REVIEW Owner tag contains a nested id
        "Storage Class" => storage_class: String,
    }
}
