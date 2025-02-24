use google_apis_common::GetToken;
use hyper::body::Body;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use yup_oauth2::{
    ApplicationDefaultCredentialsAuthenticator, ApplicationDefaultCredentialsFlowOpts,
    authenticator::ApplicationDefaultCredentialsTypes,
};

use crate::error::Error;

/// Default hyper client using platform-verifier for TLS
pub fn hyper_client<B>() -> Client<HttpsConnector<HttpConnector>, B>
where
    B: Body + Send,
    B::Data: Send,
{
    Client::builder(TokioExecutor::new()).build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_platform_verifier()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build(),
    )
}

pub async fn get_google_default_creds() -> Result<impl GetToken + 'static, Error> {
    let opts = ApplicationDefaultCredentialsFlowOpts::default();
    match ApplicationDefaultCredentialsAuthenticator::builder(opts).await {
        ApplicationDefaultCredentialsTypes::ServiceAccount(auth) => Ok(auth.build().await?),
        ApplicationDefaultCredentialsTypes::InstanceMetadata(auth) => Ok(auth.build().await?),
    }
}
