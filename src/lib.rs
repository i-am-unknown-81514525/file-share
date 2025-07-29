use worker::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[durable_object]
pub struct FileShare {
    expire_at: f64,
    state: State,
    env: Env
}

const STORAGE_DURATION: u32 = 300;

impl DurableObject for FileShare {
    fn new(state: State, env: Env) -> Self {
        Self {
            expire_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64() + (STORAGE_DURATION as f64),
            state: state,
            env: env
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        if (req.url()?.path() == "set_data") {
            self.set_data(& mut req).await?;
            self.set_alarm().await;
        }
        Response::from_bytes(self.state.storage().get::<Vec<u8>>("content").await?)
    }

    async fn alarm(&self) -> Result<Response> {
        Response::empty()
    }
}

impl FileShare {
    async fn set_data(&self, req: &mut Request) -> Result<()> {
        self.state.storage().put("content", req.bytes().await.unwrap_or(vec![])).await?;
        Ok(())
    }
    async fn set_alarm(&self) {
        match self.state.storage().get_alarm().await.unwrap_or(None) {
            Some(v) => (),
            None => {
                let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64() + (STORAGE_DURATION as f64);
                if (timestamp < self.expire_at) {
                    let diff = self.expire_at - timestamp;
                    let _ = self.state.storage().set_alarm(Duration::from_secs_f64(diff)).await;
                }
            }
        }
    }
}

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
            stub.fetch_with_request(Request::new("set_data", Method::Post)?).await.unwrap();
            
            Response::ok("ok")
        })
        .run(req, env)
        .await
}
