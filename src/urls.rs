use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::Mutex;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(untagged)]
pub enum Urls {
    Url(Url),
    UrlFailover(UrlFailover),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlFailover {
    #[serde(default)]
    #[serde(skip_serializing)]
    next: Arc<Mutex<usize>>,
    failover: Vec<Url>,
}

impl Hash for UrlFailover {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.failover.hash(state);
    }
}

impl From<Url> for UrlFailover {
    fn from(item: Url) -> Self {
        let mut vec = Vec::new();
        vec.push(item);
        UrlFailover {
            next: Arc::new(Mutex::new(0)),
            failover: vec,
        }
    }
}

impl<'a> UrlFailover {
    pub fn next(&'a self) -> &'a Url {
        let mut current = self.next.lock().unwrap();
        let len = self.failover.len();
        let next = match current {
            mut x if *x == len - 1 => {
                *x = 0;
                0
            }
            _ => {
                *current = *current + 1;
                *current
            }
        };
        let url = self.failover.get(next).unwrap();
        url
    }

    pub fn current(&'a self) -> &'a Url {
        let current = self.next.lock().unwrap();
        let url = self.failover.get(*current).unwrap();
        log::debug!("UrlFailover getting current url: {}", &url);
        url
    }

    pub fn path(&self) -> &str {
        log::trace!("UrlFailover getting path");
        self.current().path()
    }
}

impl Urls {
    pub async fn path(&self) -> &str {
        match self {
            Urls::Url(url) => url.path(),
            Urls::UrlFailover(urlfailover) => urlfailover.path(),
        }
    }
}

impl fmt::Display for UrlFailover {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        log::trace!("Printing out UrlFailover");
        write!(f, "{}", self.current())
    }
}

impl fmt::Display for Urls {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        log::trace!("Printing out enum Urls");
        match self {
            Urls::Url(url) => write!(f, "{}", url),
            Urls::UrlFailover(urlfailover) => write!(f, "{}", urlfailover),
        }
    }
}
