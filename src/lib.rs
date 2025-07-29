use worker::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[durable_object]
pub struct FileShare {
    content: Vec<u8>,
    expire_at: f64,
    state: State,
    env: Env
}

const STORAGE_DURATION: u32 = 300;

impl DurableObject for FileShare {
    fn new(state: State, env: Env) -> Self {
        Self {
            content: vec![],
            expire_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64() + (STORAGE_DURATION as f64),
            state: state,
            env: env
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        Response::from_bytes((*self).content.clone())
    }

    async fn alarm(&self) -> Result<Response> {
        Response::empty()
    }
}

// impl FileShare {
//     async fn set_alarm(&self) {
//         self.
//     }
// }

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new();

    router
        .post_async("/upload",  |mut req, ctx| async move {
            let content = req.bytes().await.unwrap_or(vec![]);
            let namespace = ctx.durable_object("file_share")?;
            let mut instance_id = String::from(req.url()?.path().to_owned());
            instance_id = match instance_id.strip_prefix("/upload") {
                Some(v) => String::from(instance_id),
                None => instance_id
            };
            let item = namespace.id_from_string(&instance_id.as_str())?;
            let stub = item.get_stub()?;
            
            Response::ok("ok")
        })
        .run(req, env)
        .await
}
