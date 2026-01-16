//! DOM query functionality (getElementById, getElementsByClassName, etc.)

use crate::node::NodeId;
use crate::tree::DomTree;

/// Trait for querying the DOM
pub trait Queryable {
    /// Find an element by its ID attribute
    fn get_element_by_id(&self, id: &str) -> Option<NodeId>;

    /// Find elements by tag name
    fn get_elements_by_tag_name(&self, tag_name: &str) -> Vec<NodeId>;

    /// Find elements by class name
    fn get_elements_by_class_name(&self, class_name: &str) -> Vec<NodeId>;
}

impl Queryable for DomTree {
    fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        let descendants = self.descendants(self.document_id());
        for node_id in descendants {
            if let Some(node) = self.get(node_id) {
                if let Some(elem) = node.as_element() {
                    if elem.id() == Some(id) {
                        return Some(node_id);
                    }
                }
            }
        }
        None
    }

    fn get_elements_by_tag_name(&self, tag_name: &str) -> Vec<NodeId> {
        let tag_lower = tag_name.to_ascii_lowercase();
        let descendants = self.descendants(self.document_id());
        descendants
            .into_iter()
            .filter(|&node_id| {
                self.get(node_id)
                    .and_then(|n| n.as_element())
                    .map(|e| e.tag_name == tag_lower)
                    .unwrap_or(false)
            })
            .collect()
    }

    fn get_elements_by_class_name(&self, class_name: &str) -> Vec<NodeId> {
        let descendants = self.descendants(self.document_id());
        descendants
            .into_iter()
            .filter(|&node_id| {
                self.get(node_id)
                    .and_then(|n| n.as_element())
                    .map(|e| e.has_class(class_name))
                    .unwrap_or(false)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_element_by_id() {
        let mut tree = DomTree::new();
        let html = tree.create_element("html");
        let div = tree.create_element("div");

        // Set the ID
        tree.get_mut(div).unwrap().as_element_mut().unwrap().set_attribute("id", "test");

        tree.append_child(tree.document_id(), html).unwrap();
        tree.append_child(html, div).unwrap();

        assert_eq!(tree.get_element_by_id("test"), Some(div));
        assert_eq!(tree.get_element_by_id("nonexistent"), None);
    }
}
