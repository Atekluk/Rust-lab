#[derive(Debug)]
pub struct New;

#[derive(Debug)]
pub struct Unmoderated;

#[derive(Debug)]
pub struct Published;

#[derive(Debug)]
pub struct Deleted;

#[derive(Debug)]
pub struct Post<S> {
    content: String,
    state: std::marker::PhantomData<S>,
}

impl Post<New> {
    pub fn new(content: &str) -> Self {
        Post {
            content: content.to_string(),
            state: std::marker::PhantomData,
        }
    }

    pub fn publish(self) -> Post<Unmoderated> {
        println!("Перехід: New -> Unmoderated");
        Post {
            content: self.content,
            state: std::marker::PhantomData,
        }
    }
}

impl Post<Unmoderated> {
    pub fn allow(self) -> Post<Published> {
        println!("Перехід: Unmoderated -> Published");
        Post {
            content: self.content,
            state: std::marker::PhantomData,
        }
    }

    pub fn deny(self) -> Post<Deleted> {
        println!("Перехід: Unmoderated -> Deleted");
        Post {
            content: self.content,
            state: std::marker::PhantomData,
        }
    }
}

impl Post<Published> {
    pub fn delete(self) -> Post<Deleted> {
        println!("Перехід: Published -> Deleted");
        Post {
            content: self.content,
            state: std::marker::PhantomData,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

// Тести
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_lifecycle_allow() {
        let post = Post::new("My first post");
        let unmoderated = post.publish();
        let published = unmoderated.allow();
        let _deleted = published.delete();
    }

    #[test]
    fn test_lifecycle_deny() {
        let post = Post::new("Bad post");
        let unmoderated = post.publish();
        let _deleted = unmoderated.deny();
    }


}