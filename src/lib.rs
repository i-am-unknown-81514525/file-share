use std::time::Duration;
use worker::*;

#[durable_object]
pub struct FileShare {
    state: State,
    #[allow(dead_code)]
    env: Env,
}

const STORAGE_DURATION: u32 = 300;

impl DurableObject for FileShare {
    fn new(state: State, env: Env) -> Self {
        Self {
            state: state,
            env: env,
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match req.url()?.path() {
            "/set_data" => {
                self.set_data(&mut req).await?;
                self.set_ttl(STORAGE_DURATION).await?;
                self.set_alarm().await?;
                self.set_key(&mut req).await?;
            }
            "/delete" => {
                self.state.storage().delete_all().await?;
            }
            "/update_ttl" => {
                self.set_ttl(STORAGE_DURATION).await?;
                self.force_set_alarm().await?;
            }
            "/is_active" => {
                return Response::ok(match self.state.storage().get_alarm().await? {
                    Some(v) => v.to_string(),
                    None => "false".to_string(),
                })
            }
            "/get_data" => {
                return match self.state.storage().get::<Vec<u8>>("content").await {
                    Ok(r) => Response::from_bytes(r),
                    Err(_) => Ok(Response::ok("")?.with_status(404)),
                }
            }
            _ => {}
        }
        Response::ok("ok")
    }

    async fn alarm(&self) -> Result<Response> {
        console_log!(
            "Deleting item for id: {}",
            match self.state.id().name() {
                Some(v) => v,
                None => self
                    .state
                    .storage()
                    .get("key")
                    .await
                    .unwrap_or("null".to_string()),
            }
        );
        self.state.storage().delete_all().await?;
        Response::empty()
    }
}

impl FileShare {
    #[inline]
    async fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.state
            .storage()
            .put("expire_at", js_sys::Date::now() / 1000.0 + (ttl as f64))
            .await?;
        Ok(())
    }

    async fn set_key(&self, req: &mut Request) -> Result<()> {
        match req.headers().get("key") {
            Ok(Some(v)) => self.state.storage().put("key", v).await?,
            _ => {}
        };
        Ok(())
    }

    async fn set_data(&self, req: &mut Request) -> Result<()> {
        self.state
            .storage()
            .put("content", req.bytes().await.unwrap_or(vec![]))
            .await?;
        Ok(())
    }
    async fn set_alarm(&self) -> Result<()> {
        match self.state.storage().get_alarm().await.unwrap_or(None) {
            Some(_) => (),
            None => self.force_set_alarm().await?,
        };
        Ok(())
    }
    async fn force_set_alarm(&self) -> Result<()> {
        let timestamp = js_sys::Date::now() / 1000.0;
        let expire_at = self.state.storage().get::<f64>("expire_at").await?;
        if timestamp < expire_at {
            let diff = expire_at - timestamp;
            let _ = self
                .state
                .storage()
                .set_alarm(Duration::from_secs_f64(diff))
                .await;
        };
        Ok(())
    }
}

#[inline]
fn get_rand() -> u32 {
    ((js_sys::Math::random() * (1e9 - 1e8 as f64)) as u32) + (1e8 as u32)
}

fn strip(string: &String, part: Vec<&str>) -> String {
    let mut out_string: String = string.clone();
    for v in part.iter() {
        out_string = match out_string.strip_prefix(v) {
            Some(stripped) => String::from(stripped),
            None => out_string,
        };
    }
    out_string
}

#[inline]
fn get_fully_qualified(instance_id: &String, file_id: &String) -> String {
    format!("{}:::{}", instance_id, file_id)
}

#[inline]
fn create_id(instance_id: &String) -> (String, String) {
    let file_id = get_rand().to_string();
    let out = get_fully_qualified(&instance_id, &file_id);
    (file_id, out)
}

async fn check_active(stub: &Stub) -> Result<i64> {
    match stub
        .fetch_with_str("https://worker/is_active")
        .await?
        .text()
        .await?
        .as_str()
    {
        "false" => Ok(-1),
        other => Ok(other.parse::<i64>().unwrap()),
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();

    router
        .post_async("/upload/:instance_id", |mut req, ctx| async move {
            let content = req.bytes().await.unwrap_or(vec![]);
            let namespace = ctx.durable_object("file_share")?;
            let mut instance_id = String::from(req.url()?.path().to_owned());
            instance_id = strip(&instance_id, vec!["/upload", "/"]);
            let (mut file_id, mut fully_qualified_id) = create_id(&instance_id);
            let mut stub = namespace.id_from_name(&fully_qualified_id.as_str())?.get_stub()?;
            let mut result = check_active(&stub).await?;
            while result != -1 {
                console_warn!("Attempt to use id: {} but already used by another item, with {}ms remaining before expire", fully_qualified_id, result);
                (file_id,  fully_qualified_id) = create_id(&instance_id);
                stub = namespace.id_from_name(&fully_qualified_id.as_str())?.get_stub()?;
                result = check_active(&stub).await?;
            }
            let header: Headers = Headers::new();
            header.append("key", fully_qualified_id.as_str())?;
            stub.fetch_with_request(Request::new_with_init(
                "https://worker/set_data",
                RequestInit::new()
                    .with_body(Some(js_sys::Uint8Array::from(content.as_slice()).into()))
                    .with_method(Method::Post)
                    .with_headers(header),
            )?)
            .await?;
            console_log!(
                "Uploaded with id: {}, Data size: {}",
                fully_qualified_id,
                content.len(),
            );
            console_log!("Data: {}", String::from_utf8_lossy(content.as_slice()));
            Response::ok(file_id)
        })
        .get_async("/download/:fully_qualified", |req, ctx| async move {
            let namespace = ctx.durable_object("file_share")?;
            let mut fully_qualified_id = String::from(req.url()?.path().to_owned());
            fully_qualified_id = strip(&instance_id, vec!["/download", "/"]);
            let item = namespace.id_from_name(&fully_qualified_id)?;
            let stub = item.get_stub()?;
            console_log!(
            	"Attempt to access file with id: {}",
             	fully_qualified_id,
            );
            return stub.fetch_with_str("https://worker/get_data").await;
        })
        .run(req, env)
        .await
}
