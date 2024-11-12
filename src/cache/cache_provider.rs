use redis::{Client, ToRedisArgs};
use std::option::Option;

#[derive(Clone)]
pub struct CacheProvider {
    redis_client: Option<Client>,
}

impl CacheProvider {
    pub fn new(url: Option<String>) -> Self {
        if let Some(url) = url {
            let redis_client = Client::open(url).unwrap();
            return CacheProvider {
                redis_client: Some(redis_client),
            };
        }

        CacheProvider {
            redis_client: None,
        }
    }

    pub fn set<T: ToRedisArgs>(&mut self, key: &str, value: &T) {
        if let Some(mut conn) = self.redis_client.clone() {
            redis::cmd("SET").arg(key).arg(value).exec(&mut conn).expect("Failed to set");
        }
    }

    pub fn get_bytes(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(mut conn) = self.redis_client.clone() {
            if let Some(exists) = redis::cmd("EXISTS").arg(key).query::<u8>(&mut conn).ok() {
                if exists == 1 {
                    return redis::cmd("GET").arg(key).query::<Vec<u8>>(&mut conn).ok();
                }
            }
        }
        None
    }
}
