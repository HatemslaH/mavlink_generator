use roxmltree::Node;

pub fn child_element<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    tag: &str,
) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name(tag))
}

pub fn child_text(node: Node<'_, '_>, tag: &str) -> Option<String> {
    child_element(node, tag).map(element_text)
}

pub fn element_text(node: Node<'_, '_>) -> String {
    node.text().unwrap_or_default().to_string()
}

pub fn descendants_named<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    tag: &str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.descendants()
        .filter(move |n| n.is_element() && n.has_tag_name(tag))
}

pub fn attr<'a, 'input: 'a>(node: Node<'a, 'input>, name: &str) -> Option<&'a str> {
    node.attribute(name)
}

pub fn cast_as_bool(value: Option<&str>, default: bool) -> crate::error::Result<bool> {
    match value {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(crate::error::GeneratorError::Format(format!(
            "Expected true or false but got \"{other}\""
        ))),
    }
}
