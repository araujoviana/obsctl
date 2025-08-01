// FIXME failed calls spit different xml structures

use serde::Serialize;

// Creates a struct with the repeated fields in the XML response
macro_rules! xml_table {
    ($struct_name:ident { $($renamed_field:expr => $table_field:ident : $t:ty),* $(,)? }) => {
        #[derive(tabled::Tabled)]
        pub struct $struct_name {
            $( // Repeats this for every field
                #[tabled(rename = $renamed_field)]
                pub $table_field: $t,
            )*
        }
    };
}
/// Parses XML into a vector of structs.
#[macro_export]
macro_rules! xml_to_struct_vec {
    (
        $table_type:ident => $repeated_field:literal in $xml:expr, { $($xml_tag:ident => $field:ident),* $(,)? }
    ) => {{
        // Unless the API somehow returns invalid XML, this can't break
        let doc = roxmltree::Document::parse($xml).unwrap();

        // Iterate over elements matching the repeated field name
        doc.descendants()
            .filter(|n| n.has_tag_name($repeated_field))
            .map(|node| {
                // Extract the text of each child element matching the given tag name
                $(
                    let $field = node
                        .descendants()
                        .find(|n| n.has_tag_name(stringify!($xml_tag)))
                        .and_then(|n| n.text())
                        .unwrap_or("")
                        .to_string();
                )*
                // Construct an instance of the struct using the extracted fields
                $table_type {
                    $(
                        $field: $field,
                    )*
                }
            })
            // Collect all constructed structs into a vector
            .collect::<Vec<$table_type>>()
    }};
}

// REVIEW these are only useful for requests that return XML content

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

// Multipart uploading

// The entire multipart upload is composed of parts
#[derive(Serialize)]
pub struct CompleteMultipartUpload {
    #[serde(rename = "Part")]
    pub parts: Vec<Part>,
}

// Part of the whole upload
#[derive(Serialize)]
pub struct Part {
    #[serde(rename = "PartNumber")]
    pub part_number: u32,
    #[serde(rename = "ETag")]
    pub etag: String,
}
