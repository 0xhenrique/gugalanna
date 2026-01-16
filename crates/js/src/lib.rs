//! Gugalanna JavaScript Engine
//!
//! JavaScript execution via QuickJS with DOM bindings.

mod console;
mod error;

pub use error::JsError;

use std::cell::RefCell;
use std::rc::Rc;

use gugalanna_dom::{DomTree, NodeId, Queryable};
use rquickjs::{Context, Function, Object, Runtime};

/// Shared reference to the DOM tree
pub type SharedDom = Rc<RefCell<DomTree>>;

/// JavaScript runtime wrapper
pub struct JsRuntime {
    runtime: Runtime,
    context: Context,
    dom: Option<SharedDom>,
}

impl JsRuntime {
    /// Create a new JavaScript runtime
    pub fn new() -> Result<Self, JsError> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        // Register console
        context.with(|ctx| {
            console::register_console(&ctx)
        })?;

        Ok(Self {
            runtime,
            context,
            dom: None,
        })
    }

    /// Create a new runtime with DOM bindings
    pub fn with_dom(dom: DomTree) -> Result<Self, JsError> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        let shared_dom = Rc::new(RefCell::new(dom));

        // Register console
        context.with(|ctx| {
            console::register_console(&ctx)
        })?;

        // Register simplified DOM API
        let dom_clone = shared_dom.clone();
        context.with(|ctx| {
            register_dom_api(&ctx, dom_clone).map_err(|e| {
                eprintln!("Failed to register DOM API: {:?}", e);
                e
            })
        })?;

        Ok(Self {
            runtime,
            context,
            dom: Some(shared_dom),
        })
    }

    /// Get a reference to the DOM tree
    pub fn dom(&self) -> Option<&SharedDom> {
        self.dom.as_ref()
    }

    /// Evaluate JavaScript code and return the result as a JsValue
    pub fn eval(&self, code: &str) -> Result<JsValue, JsError> {
        self.context.with(|ctx| {
            let result: rquickjs::Value = ctx.eval(code)?;
            Ok(convert_value(&result))
        })
    }

    /// Evaluate JavaScript code without returning a value
    pub fn exec(&self, code: &str) -> Result<(), JsError> {
        self.context.with(|ctx| {
            let _: () = ctx.eval(code)?;
            Ok(())
        })
    }

    /// Execute a script from a file (for <script> tags)
    pub fn exec_script(&self, code: &str, _filename: &str) -> Result<(), JsError> {
        self.exec(code)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create JS runtime")
    }
}

/// Register simplified DOM API
fn register_dom_api(ctx: &rquickjs::Ctx<'_>, dom: SharedDom) -> Result<(), rquickjs::Error> {
    let globals = ctx.globals();

    let document = Object::new(ctx.clone())?;

    // document.getElementById returns element ID or -1
    let dom_clone = dom.clone();
    document.set(
        "_getElementId",
        Function::new(ctx.clone(), move |id: String| -> i32 {
            let dom = dom_clone.borrow();
            dom.get_element_by_id(&id)
                .map(|nid| nid.0 as i32)
                .unwrap_or(-1)
        })?,
    )?;

    // document.getElementsByTagName returns array of IDs
    let dom_clone = dom.clone();
    document.set(
        "_getElementsByTagName",
        Function::new(ctx.clone(), move |tag: String| -> Vec<i32> {
            let dom = dom_clone.borrow();
            dom.get_elements_by_tag_name(&tag)
                .into_iter()
                .map(|nid| nid.0 as i32)
                .collect()
        })?,
    )?;

    // document.getElementsByClassName returns array of IDs
    let dom_clone = dom.clone();
    document.set(
        "_getElementsByClassName",
        Function::new(ctx.clone(), move |class: String| -> Vec<i32> {
            let dom = dom_clone.borrow();
            dom.get_elements_by_class_name(&class)
                .into_iter()
                .map(|nid| nid.0 as i32)
                .collect()
        })?,
    )?;

    // document.createElement returns new element ID
    let dom_clone = dom.clone();
    document.set(
        "_createElement",
        Function::new(ctx.clone(), move |tag: String| -> i32 {
            let mut dom = dom_clone.borrow_mut();
            dom.create_element(&tag).0 as i32
        })?,
    )?;

    // document.createTextNode returns new text node ID
    let dom_clone = dom.clone();
    document.set(
        "_createTextNode",
        Function::new(ctx.clone(), move |text: String| -> i32 {
            let mut dom = dom_clone.borrow_mut();
            dom.create_text(&text).0 as i32
        })?,
    )?;

    // _getTagName
    let dom_clone = dom.clone();
    document.set(
        "_getTagName",
        Function::new(ctx.clone(), move |node_id: i32| -> String {
            let dom = dom_clone.borrow();
            let nid = NodeId::new(node_id as u32);
            dom.get(nid)
                .and_then(|n| n.as_element())
                .map(|e| e.tag_name.to_uppercase())
                .unwrap_or_default()
        })?,
    )?;

    // _getAttribute
    let dom_clone = dom.clone();
    document.set(
        "_getAttribute",
        Function::new(ctx.clone(), move |node_id: i32, name: String| -> String {
            let dom = dom_clone.borrow();
            let nid = NodeId::new(node_id as u32);
            dom.get(nid)
                .and_then(|n| n.as_element())
                .and_then(|e| e.get_attribute(&name))
                .map(|s| s.to_string())
                .unwrap_or_default()
        })?,
    )?;

    // _setAttribute
    let dom_clone = dom.clone();
    document.set(
        "_setAttribute",
        Function::new(ctx.clone(), move |node_id: i32, name: String, value: String| {
            let mut dom = dom_clone.borrow_mut();
            let nid = NodeId::new(node_id as u32);
            dom.set_attribute(nid, &name, &value);
        })?,
    )?;

    // _appendChild
    let dom_clone = dom.clone();
    document.set(
        "_appendChild",
        Function::new(ctx.clone(), move |parent_id: i32, child_id: i32| {
            let mut dom = dom_clone.borrow_mut();
            let parent = NodeId::new(parent_id as u32);
            let child = NodeId::new(child_id as u32);
            let _ = dom.append_child(parent, child);
        })?,
    )?;

    // _getTextContent
    let dom_clone = dom.clone();
    document.set(
        "_getTextContent",
        Function::new(ctx.clone(), move |node_id: i32| -> String {
            let dom = dom_clone.borrow();
            let nid = NodeId::new(node_id as u32);
            dom.text_content(nid)
        })?,
    )?;

    globals.set("document", document)?;

    // Now inject JavaScript wrappers to create a nicer API
    let wrapper_code = r#"
        (function() {
            // Element wrapper class
            function Element(nodeId) {
                this.__nodeId = nodeId;
            }

            Object.defineProperty(Element.prototype, 'tagName', {
                get: function() { return document._getTagName(this.__nodeId); }
            });

            Object.defineProperty(Element.prototype, 'id', {
                get: function() { return document._getAttribute(this.__nodeId, 'id'); },
                set: function(v) { document._setAttribute(this.__nodeId, 'id', v); }
            });

            Object.defineProperty(Element.prototype, 'className', {
                get: function() { return document._getAttribute(this.__nodeId, 'class'); },
                set: function(v) { document._setAttribute(this.__nodeId, 'class', v); }
            });

            Object.defineProperty(Element.prototype, 'textContent', {
                get: function() { return document._getTextContent(this.__nodeId); }
            });

            Element.prototype.getAttribute = function(name) {
                var val = document._getAttribute(this.__nodeId, name);
                return val === '' ? null : val;
            };

            Element.prototype.setAttribute = function(name, value) {
                document._setAttribute(this.__nodeId, name, String(value));
            };

            Element.prototype.appendChild = function(child) {
                document._appendChild(this.__nodeId, child.__nodeId);
                return child;
            };

            // Document API wrappers
            document.getElementById = function(id) {
                var nodeId = document._getElementId(id);
                return nodeId >= 0 ? new Element(nodeId) : null;
            };

            document.getElementsByTagName = function(tag) {
                var ids = document._getElementsByTagName(tag);
                return ids.map(function(id) { return new Element(id); });
            };

            document.getElementsByClassName = function(cls) {
                var ids = document._getElementsByClassName(cls);
                return ids.map(function(id) { return new Element(id); });
            };

            document.createElement = function(tag) {
                return new Element(document._createElement(tag));
            };

            document.createTextNode = function(text) {
                return new Element(document._createTextNode(text));
            };

            document.querySelector = function(selector) {
                if (selector.charAt(0) === '#') {
                    return document.getElementById(selector.slice(1));
                }
                if (selector.charAt(0) === '.') {
                    var els = document.getElementsByClassName(selector.slice(1));
                    return els.length > 0 ? els[0] : null;
                }
                var els = document.getElementsByTagName(selector);
                return els.length > 0 ? els[0] : null;
            };

            // Store Element constructor globally
            globalThis.Element = Element;
        })();
    "#;

    ctx.eval::<(), _>(wrapper_code).map_err(|e| {
        eprintln!("JS wrapper error: {:?}", e);
        // Try to get the actual JS error
        let ex = ctx.catch();
        if let Some(err) = ex.as_exception() {
            if let Some(msg) = err.message() {
                eprintln!("JS Exception: {}", msg);
            }
            if let Some(stack) = err.stack() {
                eprintln!("Stack: {}", stack);
            }
        }
        e
    })?;

    Ok(())
}

/// JavaScript value representation
#[derive(Debug, Clone)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsValue>),
    Object,
    Function,
}

impl JsValue {
    /// Check if the value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            JsValue::Undefined | JsValue::Null => false,
            JsValue::Boolean(b) => *b,
            JsValue::Number(n) => *n != 0.0 && !n.is_nan(),
            JsValue::String(s) => !s.is_empty(),
            JsValue::Array(_) | JsValue::Object | JsValue::Function => true,
        }
    }

    /// Try to convert to a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to convert to a number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to convert to a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Convert a QuickJS value to a JsValue
fn convert_value(value: &rquickjs::Value) -> JsValue {
    use rquickjs::Type;

    match value.type_of() {
        Type::Undefined => JsValue::Undefined,
        Type::Null => JsValue::Null,
        Type::Bool => value
            .as_bool()
            .map(JsValue::Boolean)
            .unwrap_or(JsValue::Undefined),
        Type::Int => value
            .as_int()
            .map(|n| JsValue::Number(n as f64))
            .unwrap_or(JsValue::Undefined),
        Type::Float => value
            .as_float()
            .map(JsValue::Number)
            .unwrap_or(JsValue::Undefined),
        Type::String => value
            .as_string()
            .and_then(|s| s.to_string().ok())
            .map(JsValue::String)
            .unwrap_or(JsValue::Undefined),
        Type::Array => {
            if let Some(arr) = value.as_array() {
                let items: Vec<JsValue> = arr
                    .iter::<rquickjs::Value>()
                    .filter_map(|r| r.ok())
                    .map(|v| convert_value(&v))
                    .collect();
                JsValue::Array(items)
            } else {
                JsValue::Array(vec![])
            }
        }
        Type::Object => JsValue::Object,
        Type::Function => JsValue::Function,
        _ => JsValue::Undefined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_eval() {
        let runtime = JsRuntime::new().unwrap();
        let result = runtime.eval("1 + 2").unwrap();
        assert_eq!(result.as_number(), Some(3.0));
    }

    #[test]
    fn test_string_eval() {
        let runtime = JsRuntime::new().unwrap();
        let result = runtime.eval("'hello' + ' ' + 'world'").unwrap();
        assert_eq!(result.as_str(), Some("hello world"));
    }

    #[test]
    fn test_console_log() {
        let runtime = JsRuntime::new().unwrap();
        // This should not panic
        runtime.exec("console.log('Hello from JS!')").unwrap();
    }

    #[test]
    fn test_dom_access() {
        use gugalanna_html::HtmlParser;

        let html = r#"
            <html>
            <body>
                <div id="app">Hello</div>
            </body>
            </html>
        "#;

        let parser = HtmlParser::new();
        let dom = parser.parse(html).unwrap();

        let runtime = JsRuntime::with_dom(dom).unwrap();

        // Test getElementById
        let result = runtime.eval("document.getElementById('app') !== null").unwrap();
        assert_eq!(result.as_bool(), Some(true));

        // Test tagName
        let result = runtime.eval("document.getElementById('app').tagName").unwrap();
        assert_eq!(result.as_str(), Some("DIV"));
    }

    #[test]
    fn test_dom_query() {
        use gugalanna_html::HtmlParser;

        let html = r#"
            <html>
            <body>
                <p class="text">First</p>
                <p class="text">Second</p>
            </body>
            </html>
        "#;

        let parser = HtmlParser::new();
        let dom = parser.parse(html).unwrap();

        let runtime = JsRuntime::with_dom(dom).unwrap();

        // Test getElementsByTagName
        let result = runtime.eval("document.getElementsByTagName('p').length").unwrap();
        assert_eq!(result.as_number(), Some(2.0));

        // Test getElementsByClassName
        let result = runtime.eval("document.getElementsByClassName('text').length").unwrap();
        assert_eq!(result.as_number(), Some(2.0));
    }

    #[test]
    fn test_set_attribute() {
        use gugalanna_html::HtmlParser;

        let html = r#"<div id="test"></div>"#;

        let parser = HtmlParser::new();
        let dom = parser.parse(html).unwrap();

        let runtime = JsRuntime::with_dom(dom).unwrap();

        // Set attribute
        runtime.exec("document.getElementById('test').setAttribute('data-foo', 'bar')").unwrap();

        // Get attribute
        let result = runtime.eval("document.getElementById('test').getAttribute('data-foo')").unwrap();
        assert_eq!(result.as_str(), Some("bar"));
    }

    #[test]
    fn test_create_element() {
        use gugalanna_html::HtmlParser;

        let html = r#"<div id="container"></div>"#;

        let parser = HtmlParser::new();
        let dom = parser.parse(html).unwrap();

        let runtime = JsRuntime::with_dom(dom).unwrap();

        // Create and append element
        runtime.exec(r#"
            var span = document.createElement('span');
            span.setAttribute('id', 'new-element');
            document.getElementById('container').appendChild(span);
        "#).unwrap();

        // Verify element was created
        let result = runtime.eval("document.getElementById('new-element') !== null").unwrap();
        assert_eq!(result.as_bool(), Some(true));
    }
}
