use super::*;
use libc::c_void;

macro_rules! wrap_handler {
    ($handler:ident, $user_data:expr) => {{
        // NOTE: the closure actually holds a reference to the content
        // handler object, but since we pass the object to the C side this
        // ownership information gets erased.
        // It's not a problem since handler is an extern static function that
        // will remain intact even if Rust-side builder object gets freed.
        // However, it's not a case for the user data pointer, it might become
        // invalid if content handlers object that holds it gets freed before
        // a handler invocation. Therefore, we close on a local variable instead
        // of structure field.
        let user_data = $user_data;

        move |arg: &mut _| unsafe { $handler(arg, user_data) }
    }};
}

struct ExternHandler<T> {
    handler: Option<T>,
    user_data: *mut c_void,
}

impl<T> ExternHandler<T> {
    fn new(handler: Option<T>, user_data: *mut c_void) -> Self {
        ExternHandler { handler, user_data }
    }
}

pub struct ExternDocumentContentHandlers {
    doctype: ExternHandler<unsafe extern "C" fn(*mut Doctype, *mut c_void)>,
    comments: ExternHandler<unsafe extern "C" fn(*mut Comment, *mut c_void)>,
    text: ExternHandler<unsafe extern "C" fn(*mut TextChunk, *mut c_void)>,
}

impl ExternDocumentContentHandlers {
    pub fn as_safe_document_content_handlers(&self) -> DocumentContentHandlers {
        let mut handlers = DocumentContentHandlers::default();

        if let Some(handler) = self.doctype.handler {
            handlers = handlers.doctype(wrap_handler!(handler, self.doctype.user_data));
        }

        if let Some(handler) = self.comments.handler {
            handlers = handlers.comments(wrap_handler!(handler, self.comments.user_data));
        }

        if let Some(handler) = self.text.handler {
            handlers = handlers.text(wrap_handler!(handler, self.text.user_data));
        }

        handlers
    }
}

pub struct ExternElementContentHandlers {
    element: ExternHandler<unsafe extern "C" fn(*mut Element, *mut c_void)>,
    comments: ExternHandler<unsafe extern "C" fn(*mut Comment, *mut c_void)>,
    text: ExternHandler<unsafe extern "C" fn(*mut TextChunk, *mut c_void)>,
}

impl ExternElementContentHandlers {
    pub fn as_safe_element_content_handlers(&self) -> ElementContentHandlers {
        let mut handlers = ElementContentHandlers::default();

        if let Some(handler) = self.element.handler {
            handlers = handlers.element(wrap_handler!(handler, self.element.user_data));
        }

        if let Some(handler) = self.comments.handler {
            handlers = handlers.comments(wrap_handler!(handler, self.comments.user_data));
        }

        if let Some(handler) = self.text.handler {
            handlers = handlers.text(wrap_handler!(handler, self.text.user_data));
        }

        handlers
    }
}

pub struct SafeContentHandlers<'b> {
    pub document: Vec<DocumentContentHandlers<'b>>,
    pub element: Vec<(&'b Selector, ElementContentHandlers<'b>)>,
}

#[derive(Default)]
pub struct HtmlRewriterBuilder {
    document_content_handlers: Vec<ExternDocumentContentHandlers>,
    element_content_handlers: Vec<(Selector, ExternElementContentHandlers)>,
}

impl HtmlRewriterBuilder {
    pub fn get_safe_handlers(&self) -> SafeContentHandlers {
        SafeContentHandlers {
            document: self
                .document_content_handlers
                .iter()
                .map(|h| h.as_safe_document_content_handlers())
                .collect(),
            element: self
                .element_content_handlers
                .iter()
                .map(|(s, h)| (s, h.as_safe_element_content_handlers()))
                .collect(),
        }
    }
}

#[no_mangle]
pub extern "C" fn cool_thing_rewriter_builder_new() -> *mut HtmlRewriterBuilder {
    to_ptr_mut(HtmlRewriterBuilder::default())
}

#[no_mangle]
pub extern "C" fn cool_thing_rewriter_builder_add_document_content_handlers(
    builder: *mut HtmlRewriterBuilder,
    doctype_handler: Option<unsafe extern "C" fn(*mut Doctype, *mut c_void)>,
    doctype_handler_user_data: *mut c_void,
    comments_handler: Option<unsafe extern "C" fn(*mut Comment, *mut c_void)>,
    comments_handler_user_data: *mut c_void,
    text_handler: Option<unsafe extern "C" fn(*mut TextChunk, *mut c_void)>,
    text_handler_user_data: *mut c_void,
) {
    let builder = to_ref_mut!(builder);

    let handlers = ExternDocumentContentHandlers {
        doctype: ExternHandler::new(doctype_handler, doctype_handler_user_data),
        comments: ExternHandler::new(comments_handler, comments_handler_user_data),
        text: ExternHandler::new(text_handler, text_handler_user_data),
    };

    builder.document_content_handlers.push(handlers);
}

#[no_mangle]
pub extern "C" fn cool_thing_rewriter_builder_add_element_content_handlers(
    builder: *mut HtmlRewriterBuilder,
    selector: *const c_char,
    selector_len: size_t,
    element_handler: Option<unsafe extern "C" fn(*mut Element, *mut c_void)>,
    element_handler_user_data: *mut c_void,
    comments_handler: Option<unsafe extern "C" fn(*mut Comment, *mut c_void)>,
    comments_handler_user_data: *mut c_void,
    text_handler: Option<unsafe extern "C" fn(*mut TextChunk, *mut c_void)>,
    text_handler_user_data: *mut c_void,
) -> c_int {
    let selector = unwrap_or_ret_err_code! { to_str!(selector, selector_len) };
    let selector = unwrap_or_ret_err_code! { selector.parse::<Selector>() };
    let builder = to_ref_mut!(builder);

    let handlers = ExternElementContentHandlers {
        element: ExternHandler::new(element_handler, element_handler_user_data),
        comments: ExternHandler::new(comments_handler, comments_handler_user_data),
        text: ExternHandler::new(text_handler, text_handler_user_data),
    };

    builder.element_content_handlers.push((selector, handlers));

    0
}

#[no_mangle]
pub extern "C" fn cool_thing_rewriter_builder_free(builder: *mut HtmlRewriterBuilder) {
    drop(to_box!(builder));
}
