use anyhow::Result;
use hashbrown::{HashMap, HashSet};
use scraper::{Html, Node, Selector};
use std::{fs, path::Path};
use table_extractor::context::ContentHierarchy;
use table_extractor::extractors::context_v1::ContextExtractor;
use table_extractor::{
    misc::SimpleTree,
    text::{get_rich_text, get_text, RichText, RichTextElement},
};

fn get_doc(filename: &str) -> Result<Html> {
    let html_file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/resources")
        .join(filename);
    let html = fs::read_to_string(html_file)?;
    Ok(Html::parse_document(&html))
}

#[test]
fn test_locate_content_before_and_after() -> Result<()> {
    let extractor = ContextExtractor::default();

    let doc = get_doc("context/one-level.html")?;
    let selector = Selector::parse("#marker").unwrap();

    let elements = doc.select(&selector).collect::<Vec<_>>();
    assert_eq!(elements.len(), 1);

    let (tree_before, tree_after) = extractor.locate_content_before_and_after(*elements[0])?;

    let node_key = |uid| match tree_before.get_node(uid).value() {
        Node::Element(x) => format!("{}", x.name()),
        Node::Text(x) => format!("`{}`", &x[..x.len().min(20)].replace("\n", "\\n")),
        _ => format!("{}", uid),
    };

    assert!(tree_before.validate());
    assert!(tree_after.validate());
    // println!("{}", tree_before.to_string(&node_key));
    assert_eq!(
        tree_before.to_string(&node_key).trim(),
        r#"
body -> {
    `\n    `
    h1
    `\n    `
    div -> {
        `\n      abc\n      `
        span
        `\n      `
        p
        `\n      `
        span
        ` `
        a
        `\n      .\n      `
    }
}
    "#
        .trim()
    );

    Ok(())
}

#[test]
fn test_flatten_node() -> Result<()> {
    let extractor = ContextExtractor::default();

    let doc = get_doc("context/three-level.html")?;
    let selector = Selector::parse("#section-1").unwrap();

    let elements = doc.select(&selector).collect::<Vec<_>>();
    assert_eq!(elements.len(), 1);

    let mut output = Vec::new();
    extractor.flatten_node(&*elements[0], &mut output);

    // println!("{:#?}", output);
    assert_eq!(
        format!("{:#?}", output),
        r#"
[
    `<>abc <span>def</span></>`,
    `<>Content of section 1</>`,
    `<h2>Section 1.1</h2>`,
    `<>Content of section 1.1</>`,
    `<><span>hello</span> <a>World</a> .</>`,
]
    "#
        .trim()
    );

    let mut output_recur = Vec::new();
    extractor.flatten_node_recur(&*elements[0], &mut output_recur);
    assert_eq!(output, output_recur);
    Ok(())
}

#[test]
fn test_context_extractor() -> Result<()> {
    let extractor = ContextExtractor::default();

    let doc = get_doc("context/three-level.html")?;
    let selector = Selector::parse("#marker").unwrap();

    let elements = doc.select(&selector).collect::<Vec<_>>();
    assert_eq!(elements.len(), 1);

    let context = extractor.extract_context(*elements[0])?;

    println!("{:#?}", context);
    assert_eq!(
        format!("{:#?}", context),
        r#"
[
    ContentHierarchy {
        level: 0,
        heading: `<></>`,
        content_before: [
            `<>Date: Today</>`,
        ],
        content_after: [],
    },
    ContentHierarchy {
        level: 1,
        heading: `<h1>Section 1</h1>`,
        content_before: [
            `<>abc <span>def</span></>`,
            `<>Content of section 1</>`,
        ],
        content_after: [],
    },
    ContentHierarchy {
        level: 2,
        heading: `<h2>Section 1.1</h2>`,
        content_before: [
            `<>Content of section 1.1</>`,
            `<><span>hello</span> <a>World</a> .</>`,
        ],
        content_after: [],
    },
    ContentHierarchy {
        level: 3,
        heading: `<h3>Section 1.1.1</h3>`,
        content_before: [
            `<>here <span>is the section</span> <b>1.1.1</b></>`,
        ],
        content_after: [],
    },
]
"#
        .trim()
    );

    Ok(())
}
