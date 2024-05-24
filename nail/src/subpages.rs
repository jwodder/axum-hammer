use crate::consts::{SUBPAGE_BODY_LEN, SUBPAGE_NAME_LEN, SUBPAGE_QTY};
use axum::{
    body::Body,
    extract::Request,
    http::{response::Response, StatusCode},
    response::IntoResponse,
};
use rand::{
    distributions::{Alphanumeric, DistString, Distribution, Standard},
    Rng,
};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubpageService {
    path: &'static str, // Stored without leading or trailing "/"
    data: BTreeMap<String, Vec<u8>>,
}

impl SubpageService {
    pub(crate) fn new<R: Rng>(path: &'static str, rng: R) -> SubpageService {
        SubpageService {
            path: path.trim_matches('/'),
            data: gen_subpages(rng),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn index_response(&self) -> Result<Response<Body>, Infallible> {
        let mut body = String::new();
        for key in self.data.keys() {
            writeln!(&mut body, "/{}/{}", self.path, key)
                .expect("Writing to a String should not fail");
        }
        Ok(body.into_response())
    }

    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn subpage_response(&self, key: &str) -> Result<Response<Body>, Infallible> {
        if let Some(body) = self.data.get(key) {
            Ok(body.clone().into_response())
        } else {
            Ok((StatusCode::NOT_FOUND, "404\n").into_response())
        }
    }

    #[allow(clippy::unused_async)]
    pub(crate) async fn handle_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        // ↓↓ Does not have leading "/{self.path}"
        let reqpath = req.uri().path().trim_start_matches('/');
        if reqpath.is_empty() {
            self.index_response()
        } else {
            self.subpage_response(reqpath)
        }
    }
}

fn gen_subpages<R: Rng>(mut rng: R) -> BTreeMap<String, Vec<u8>> {
    let mut subpages = BTreeMap::new();
    for _ in 0..SUBPAGE_QTY {
        let name = loop {
            let name = Alphanumeric.sample_string(&mut rng, SUBPAGE_NAME_LEN);
            if !subpages.contains_key(&name) {
                break name;
            }
        };
        let body = Standard
            .sample_iter(&mut rng)
            .take(SUBPAGE_BODY_LEN)
            .collect::<Vec<_>>();
        subpages.insert(name, body);
    }
    subpages
}
