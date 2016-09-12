mod summary;
mod errors;

pub use self::summary::Summary;
pub use self::errors::{ParseError};

#[derive(Clone, Debug)]
pub struct Link {
    name: String,
    destination: String,

    children: Option<Vec<Link>>,
}

impl Link {
    pub fn new<S>(name: S, destination: S) -> Self where S: Into<String> {
        Link {
            name: name.into(),
            destination: destination.into(),

            children: None,
        }
    }

    pub fn set_children(&mut self, children: Vec<Link>) -> &mut Self {
        self.children = Some(children);
        self
    }
}
