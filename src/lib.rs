use worker::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rand::Rng;


#[durable_object]
pub struct FileShare {
    state: State,
    #[allow(dead_code)]
    env: Env
}

const STORAGE_DURATION: u32 = 300;

impl DurableObject for FileShare {
    fn new(state: State, env: Env) -> Self {
        Self {
            state: state,
            env: env
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match req.url()?.path() {
            "set_data" => {
                self.set_data(& mut req).await?;
                self.set_ttl(STORAGE_DURATION).await?;
                self.set_alarm().await?;
            },
            "delete" => {
                self.state.storage().delete_all().await?;
            },
            "update_ttl" => {
                self.set_ttl(STORAGE_DURATION).await?;
                self.force_set_alarm().await?;
            },
            "is_active" => return Response::ok(match self.state.storage().get_alarm().await? {Some(v) => v.to_string(), None => "false".to_string()}),
            "get_data" => return match self.state.storage().get::<Vec<u8>>("content").await {
                Ok(r) => Response::from_bytes(r),
                Err(_) => Ok(Response::ok("")?.with_status(404))
            },
            _ => {}
        }
        Response::ok("ok")
    }

    async fn alarm(&self) -> Result<Response> {
        self.state.storage().delete_all().await?;
        Response::empty()
    }
}

impl FileShare {
    #[inline]
    async fn set_ttl(&self, ttl: u32) -> Result<()>{
        self.state
            .storage()
            .put("expire_at", SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap()
            .as_secs_f64() + 
            (ttl as f64)
        ).await?;
        Ok(())
    }

    async fn set_data(&self, req: &mut Request) -> Result<()> {
        self.state.storage().put("content", req.bytes().await.unwrap_or(vec![])).await?;
        Ok(())
    }
    async fn set_alarm(&self) -> Result<()> {
        match self.state.storage().get_alarm().await.unwrap_or(None) {
            Some(_) => (),
            None => self.force_set_alarm().await?
        };
        Ok(())
    }
    async fn force_set_alarm(&self) -> Result<()> {
        let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();
        let expire_at = self.state.storage().get::<f64>("expire_at").await?;
        if timestamp < expire_at {
            let diff = expire_at - timestamp;
            let _ = self.state.storage().set_alarm(Duration::from_secs_f64(diff)).await;
        };
        Ok(())
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
                Some(v) => String::from(v),
                None => instance_id
            };
            instance_id = match instance_id.strip_prefix("/") {
                Some(v) => String::from(v),
                None => instance_id
            };
            let mut file_id = rand::rng().random_range(1e8..1e9).to_string(); // always 8 digit
            let mut item = namespace.id_from_name(&format!("{}:::{}", instance_id, file_id).as_str())?;
            let mut stub = item.get_stub()?;
            while stub.fetch_with_str("https://worker/is_active").await?.text().await? != "false".to_string() {
                file_id = rand::rng().random_range(1e8..1e9).to_string(); // always 8 digit
                item = namespace.id_from_name(&format!("{}:::{}", instance_id, file_id).as_str())?;
                 stub = item.get_stub()?;
            }
            stub.fetch_with_request(
                Request::new_with_init(
                    "https://worker/set_data", 
                    RequestInit::new()
                        .with_body(
                            Some(js_sys::Uint8Array::from(content.as_slice()).into())
                        )
                        .with_method(Method::Post)
                )?
            ).await?;
            // stub.fetch_with_request(Request::new("set_data", Method::Post)?).await.unwrap();
            Response::ok(file_id)
        })
        .get_async("/download", |req, ctx| async move {
            let namespace = ctx.durable_object("file_share")?;
            let mut fully_qualified_id = String::from(req.url()?.path().to_owned());
            fully_qualified_id = match fully_qualified_id.strip_prefix("/download") {
                Some(v) => String::from(v),
                None => fully_qualified_id
            };
            fully_qualified_id = match fully_qualified_id.strip_prefix("/") {
                Some(v) => String::from(v),
                None => fully_qualified_id
            };
            let item = namespace.id_from_name(&fully_qualified_id)?;
            let stub = item.get_stub()?;
            return stub.fetch_with_str("https://worker/get_data").await
        })
        .run(req, env)
        .await
}
