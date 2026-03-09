use google_cloud_auth::credentials::CacheableResource;
use google_cloud_auth::credentials::Credentials;
use google_cloud_auth::credentials::EntityTag;
use http::HeaderMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Combine {
    cache: Arc<Mutex<Cache>>,
}

impl Combine {
    pub fn new(c1: Credentials, c2: Credentials) -> Self {
        let cache = Arc::new(Mutex::new(Cache::new(c1, c2)));
        Self { cache }
    }

    pub async fn total_headers(
        &self,
        extensions: http::Extensions,
    ) -> anyhow::Result<CacheableResource<usize>> {
        let tag = extensions.get::<EntityTag>();
        let mut guard = self.cache.lock().await;
        let h1 = Self::query(&guard.c1, &guard.t1).await?;
        let h2 = Self::query(&guard.c2, &guard.t2).await?;
        let new = match (tag, h1, h2) {
            (Some(tag), CacheableResource::NotModified, CacheableResource::NotModified)
                if Some(tag) == guard.tag.as_ref() =>
            {
                return Ok(CacheableResource::NotModified);
            }
            (None | Some(_), CacheableResource::NotModified, CacheableResource::NotModified) => {
                return Ok(CacheableResource::New {
                    entity_tag: guard
                        .tag
                        .clone()
                        .expect("must have been set if c1 and c2 return NotModified"),
                    data: guard.s1 + guard.s2,
                });
            }
            (_, CacheableResource::New { entity_tag, data }, CacheableResource::NotModified) => {
                guard.update_c1(entity_tag, data)
            }
            (_, CacheableResource::NotModified, CacheableResource::New { entity_tag, data }) => {
                guard.update_c2(entity_tag, data)
            }
            (
                _,
                CacheableResource::New {
                    entity_tag: t1,
                    data: d1,
                },
                CacheableResource::New {
                    entity_tag: t2,
                    data: d2,
                },
            ) => {
                let _ = guard.update_c1(t1, d1);
                guard.update_c2(t2, d2)
            }
        };
        Ok(new)
    }

    pub async fn query(
        c: &Credentials,
        t: &Option<EntityTag>,
    ) -> anyhow::Result<CacheableResource<HeaderMap>> {
        let mut e = http::Extensions::new();
        if let Some(tag) = t {
            let _ = e.insert(tag.clone());
        };
        let headers = c.headers(e).await?;
        Ok(headers)
    }
}

#[derive(Debug)]
struct Cache {
    tag: Option<EntityTag>,

    c1: Credentials,
    t1: Option<EntityTag>,
    s1: usize,

    c2: Credentials,
    t2: Option<EntityTag>,
    s2: usize,
}

impl Cache {
    fn new(c1: Credentials, c2: Credentials) -> Self {
        Self {
            c1,
            c2,
            t1: None,
            t2: None,
            tag: None,
            s1: 0,
            s2: 0,
        }
    }

    fn update_c1(&mut self, tag: EntityTag, headers: HeaderMap) -> CacheableResource<usize> {
        self.t1 = Some(tag);
        self.s1 = headers.len();
        let new = EntityTag::new();
        self.tag = Some(new.clone());
        CacheableResource::New {
            entity_tag: new,
            data: self.s1 + self.s2,
        }
    }

    fn update_c2(&mut self, tag: EntityTag, headers: HeaderMap) -> CacheableResource<usize> {
        self.t2 = Some(tag);
        self.s2 = headers.len();
        let new = EntityTag::new();
        self.tag = Some(new.clone());
        CacheableResource::New {
            entity_tag: new,
            data: self.s1 + self.s2,
        }
    }
}
