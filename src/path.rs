use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
};
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::RwLock;

use crate::error::Error as ProximaError;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ProxyPath {
    pub path: String,
    pub vec: Vec<String>,
    pub count: Arc<RwLock<i32>>,
    pub max: i32,
}

impl ProxyPath {
    pub fn new(path: &str) -> ProxyPath {
        if path != "/" {
            // Create String so that we can manipulate it
            let mut path_string = path.to_string();
    
            // Remove prefix of /
            #[allow(clippy::iter_nth_zero)]
            if let Some('/') = path_string.chars().nth(0) {
                log::trace!("Stripping / prefix");
                path_string.remove(0);
            };
    
            // Remove suffix of /
            if let Some('/') = path_string.chars().last() {
                log::trace!("Stripping / suffix");
                path_string.pop();
            };
    
            log::trace!("Final path string: {}", &path_string);
            let vec: Vec<String> = path_string.split('/').map(str::to_string).collect();
    
            log::debug!("Final path array: {:?}", &vec);
            let max = vec.len() - 1;
    
            ProxyPath {
                path: path.to_string(),
                vec: vec,
                count: Arc::new(RwLock::new(-1)),
                max: max as i32,
            }
        } else {
            ProxyPath {
                path: path.to_string(),
                vec: vec![path.to_string()],
                count: Arc::new(RwLock::new(-1)),
                max: 1
            }
        }
    }

    pub fn next(&mut self) -> Result<(), ProximaError> {
        let mut count = self.count.write().unwrap();
        if *count < self.max {
            *count = *count + 1;
            drop(count);
            Ok(())
        } else {
            Err(ProximaError::UnknownProxy)
        }
    }

    pub fn count(&self) -> i32 {
        let count = self.count.read().unwrap();
        *count
    }

    pub fn set_count(&self, count: i32) -> Result<(), ProximaError> {
        if count > self.max {
            Err(ProximaError::PathCount)
        } else {
            let mut current = self.count.write().unwrap();
            *current = count;
            Ok(())
        }
    }

    pub fn forward(&mut self, key: &str) -> Result<(), ProximaError> {
        log::debug!("Setting {} forward in {}", key, self.path());
        let slashes = key.matches('/').count() as i32;
        self.set_count(slashes)?;
        Ok(())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    //    pub fn prefix(&self) -> String {
    //        if self.count() == -1 {
    //            return "".to_string()
    //        };
    //        let slice = self.vec[..self.count() as usize].to_vec().join("/");
    //        match slice.as_str() {
    //            "" => "".to_string(),
    //            _ => slice
    //        }
    //    }

    pub fn current(&self) -> String {
        let count = match self.count() {
            -1 => 0,
            _ => self.count(),
        };
        self.vec.get(count as usize).unwrap().to_string()
    }

    pub fn next_hop(&self) -> Option<String> {
        let index = self.count() + 1;
        match self.vec.get(index as usize) {
            Some(h) => Some(h.to_string()),
            None => None,
        }
    }

    pub fn next_key(&self) -> Option<String> {
        if self.count() == -1 {
            return None;
        };
        let count = self.count() + 2;
        log::trace!("next_key: {} vs {} ?", &count, self.max + 1);
        if count <= self.max + 1 {
            log::trace!("next_key: {:?}", self.vec);
            let slice = self.vec[..count as usize].to_vec().join("/");
            log::trace!(
                "next_key, getting path slice with count of {}: {}",
                &count,
                &slice.as_str()
            );
            match slice.as_str() {
                "" => None,
                _ => Some(slice),
            }
        } else {
            None
        }
    }

    pub fn key(&self) -> Option<String> {
        if self.count() == -1 {
            log::debug!("path count is -1, returning None");
            return None;
        };
        let count = self.count() + 1;
        let slice = self.vec[..count as usize].to_vec().join("/");
        log::trace!(
            "key, getting path slice with count of {}: {}",
            &count,
            &slice.as_str()
        );
        match slice.as_str() {
            "" => None,
            _ => Some(slice),
        }
    }

    //    pub fn prefix_dot_notated(&self) -> String {
    //        let end = self.count() as usize;
    //        let slice = self.vec[..end].to_vec().join(".");
    //        slice
    //    }

    pub fn suffix(&self) -> String {
        log::debug!("{} vs {}", self.count(), self.max);
        if self.count() < self.max {
            let start = self.count() + 1;
            let slice = self.vec[start as usize..].to_vec().join("/");
            log::debug!("Returning suffix of {}", &slice);
            slice
        } else {
            "".to_string()
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for ProxyPath
where
    B: Send,
{
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let uri = req.uri().to_owned();
        Ok(ProxyPath::new(uri.path()))
    }
}
