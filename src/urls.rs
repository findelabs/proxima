use std::hash::{Hash, Hasher};
use url::Url;
use std::sync::Mutex;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::fmt;

type UrlList = Vec<Url>;

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
#[serde(untagged)]
pub enum Urls {
    Url(Url),
    UrlLB(UrlLB)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlLB {
    #[serde(default)]
    #[serde(skip_serializing)]
    next: Arc<Mutex<usize>>,
    members: Vec<Url>
}

impl Hash for UrlLB {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let next= self.next.lock().unwrap();
        next.hash(state);
        self.members.hash(state);
    }
}

impl From<UrlList> for UrlLB {
    fn from(item: UrlList) -> Self {
        UrlLB { 
            next: Arc::new(Mutex::new(0)),
            members: item.clone()
        }
    }
}

impl From<Url> for UrlLB {
    fn from(item: Url) -> Self {
        let mut vec = Vec::new();
        vec.push(item);
        UrlLB { 
            next: Arc::new(Mutex::new(0)),
            members: vec
        }
    }
}

impl<'a> UrlLB {
    pub fn next(&'a self) -> &'a Url {
        let mut current = self.next.lock().unwrap();
        let len = self.members.len();
        let next = match current {
            mut x if *x == len - 1 => {
                *x = 0;
                0
            },
            _ => {
                *current = *current + 1;
                *current
            }
        };
        let url = self.members.get(next).unwrap();
        url
    }

    pub fn url(&'a self) -> &'a Url {
        let current = self.next.lock().unwrap();
        let url = self.members.get(*current).unwrap();
        url
    }

    pub fn path(&self) -> &str {
        let url = self.url();
        url.path()
    }
}

impl Urls {
    pub async fn path(&self) -> &str {
        match self {
            Urls::Url(url) => url.path(),
            Urls::UrlLB(urls) => urls.path()
        }
    }
}

impl fmt::Display for UrlLB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        log::debug!("Printing out UrlLB");
        write!(f, "{}", self.next())
    }
}

impl fmt::Display for Urls {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        log::debug!("Printing out Urls");
        match self {
            Urls::Url(url) => write!(f, "{}", url),
            Urls::UrlLB(urls) => write!(f, "{}", urls)
        }
    }
}
