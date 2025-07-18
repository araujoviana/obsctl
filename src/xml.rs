use roxmltree::Document;
use tabled::{Table, Tabled, derive};

// List buckets
#[derive(Tabled)]
pub struct BucketList {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Created At")]
    creation_date: String,
    #[tabled(rename = "Location")]
    location: String,
    #[tabled(rename = "Bucket Type")]
    bucket_type: String,
}

// TODO convert to macro
pub fn parse_bucket_list(xml: &str) -> Vec<BucketList> {
    let doc = Document::parse(xml).unwrap(); // REVIEW this could go bad

    doc.descendants()
        .filter(|n| n.has_tag_name("Bucket"))
        .map(|bucket_node| {
            let name = bucket_node
                .descendants()
                .find(|n| n.has_tag_name("Name"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string();

            let creation_date = bucket_node
                .descendants()
                .find(|n| n.has_tag_name("CreationDate"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string();

            let location = bucket_node
                .descendants()
                .find(|n| n.has_tag_name("Location"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string();

            let bucket_type = bucket_node
                .descendants()
                .find(|n| n.has_tag_name("BucketType"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string();

            BucketList {
                name,
                creation_date,
                location,
                bucket_type,
            }
        })
        .collect()
}
