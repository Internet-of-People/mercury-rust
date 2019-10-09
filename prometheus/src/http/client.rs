use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

use actix_http::http::StatusCode;
use actix_web::{
    client::{Client as HttpClient, ClientResponse, SendRequestError},
    error::ParseError,
};
use failure::{format_err, Fallible};
use futures::{Future, IntoFuture};
//use log::*;

use crate::*;
use actix_http::error::PayloadError;
use claims::model::*;
use did::vault::{ProfileLabel, ProfileMetadata, ProfileVaultRecord};

pub struct VaultClient {
    root_url: String,
    reactor: RefCell<actix_rt::SystemRunner>,
}

impl VaultClient {
    pub fn new(root_url: &str) -> Self {
        Self {
            root_url: root_url.to_owned(),
            reactor: RefCell::new(actix_rt::System::new("ActixReactor")),
        }
    }

    // Note could we try using actix_web::error::Error instead?
    fn await_fut<T, E: actix_http::error::ResponseError>(
        &self,
        fut: impl Future<Item = T, Error = E>,
    ) -> Fallible<T> {
        let ret = self.reactor.borrow_mut().block_on(fut).map_err(|e| err_msg(e.to_string()))?;
        Ok(ret)
    }
}

fn did_str(did_opt: Option<ProfileId>) -> String {
    match did_opt {
        None => "_".to_owned(),
        Some(did) => did.to_string(),
    }
}

// TODO we should also log and return more response contents describing more error details

fn validate_response_status<
    T: 'static + futures::stream::Stream<Item = bytes::Bytes, Error = actix_http::error::PayloadError>,
>(
    mut response: ClientResponse<T>,
    status: StatusCode,
) -> Box<dyn Future<Item = ClientResponse<T>, Error = SendRequestError>> {
    if response.status() != status {
        warn!("Got response with unexpected status: {}", response.status());
        return Box::new(
            response
                .body()
                .and_then(|body| {
                    let body_str = String::from_utf8(body.to_vec()).map_err(|e| {
                        warn!("Failed to decode error message from response: {}", e);
                        PayloadError::EncodingCorrupted
                    });
                    warn!("Error details: {}", body_str?);
                    Ok(())
                })
                .then(|_res| Err(ParseError::Status.into())),
        );
    }
    Box::new(Ok(response).into_future())
}

impl VaultApi for VaultClient {
    fn restore_vault(&mut self, phrase: String) -> Fallible<()> {
        let url = format!("{}/vault", self.root_url);
        // TODO phrase should normally be splitted into words and sent that way,
        //      but this will work for the moment
        let req_fut = HttpClient::new().post(url).send_json(&vec![phrase]);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::CREATED))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn restore_all_profiles(&mut self) -> Fallible<RestoreCounts> {
        let url = format!("{}/vault/restore-dids", self.root_url);
        let req_fut = HttpClient::new().post(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::CREATED))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())));
        self.await_fut(fut)
    }

    fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()> {
        let url = format!("{}/vault/default-did", self.root_url);
        let req_fut = HttpClient::new().put(url).send_json(&my_profile_id.to_string());
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn get_active_profile(&self) -> Fallible<Option<ProfileId>> {
        let url = format!("{}/vault/default-did", self.root_url);
        let req_fut = HttpClient::new().get(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())))
            .and_then(|did_str_opt: Option<String>| {
                Ok(match did_str_opt {
                    None => None,
                    Some(did_str) => Some(
                        did_str
                            .parse()
                            .map_err(|e: failure::Error| SendRequestError::Body(e.into()))?,
                    ),
                })
            });
        self.await_fut(fut)
    }

    //fn list_vault_records(&self) -> Fallible<Vec<VaultEntry>> {
    fn list_vault_records(&self) -> Fallible<Vec<ProfileVaultRecord>> {
        let url = format!("{}/vault/dids", self.root_url);
        let req_fut = HttpClient::new().get(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())))
            .and_then(|entries: Vec<VaultEntry>| {
                let recs = entries
                    .iter()
                    .filter_map(|entry| {
                        // TODO we should at least log errors here
                        entry.try_into().ok()
                    })
                    .collect();
                Ok(recs)
            });
        self.await_fut(fut)
    }

    fn create_profile(&mut self, label: Option<ProfileLabel>) -> Fallible<ProfileVaultRecord> {
        let url = format!("{}/vault/dids", self.root_url);
        let req_fut = HttpClient::new().post(url).send_json(&label.unwrap_or_default());
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::CREATED))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())))
            .and_then(|entry: VaultEntry| {
                (&entry).try_into().map_err(|e: failure::Error| SendRequestError::Body(e.into()))
            });
        self.await_fut(fut)
    }

    fn get_vault_record(&self, id: Option<ProfileId>) -> Fallible<ProfileVaultRecord> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}", self.root_url, did);
        let req_fut = HttpClient::new().get(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())))
            .and_then(|entry: VaultEntry| {
                (&entry).try_into().map_err(|e: failure::Error| SendRequestError::Body(e.into()))
            });
        self.await_fut(fut)
    }

    fn set_profile_label(&mut self, id: Option<ProfileId>, label: ProfileLabel) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/label", self.root_url, did);
        let req_fut = HttpClient::new().put(url).send_json(&label);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn get_profile_metadata(&self, _id: Option<ProfileId>) -> Fallible<ProfileMetadata> {
        unimplemented!() // NOTE not present in the CLI so far
    }

    fn set_profile_metadata(
        &mut self,
        _id: Option<ProfileId>,
        _data: ProfileMetadata,
    ) -> Fallible<()> {
        unimplemented!() // NOTE not present in the CLI so far
    }

    fn get_profile_data(
        &self,
        id: Option<ProfileId>,
        _repo_kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/profiledata", self.root_url, did);
        let req_fut = HttpClient::new().get(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())));
        self.await_fut(fut)
    }

    fn revert_profile(&mut self, id: Option<ProfileId>) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/revert", self.root_url, did);
        let req_fut = HttpClient::new().post(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())));
        self.await_fut(fut)
    }

    fn publish_profile(&mut self, id: Option<ProfileId>, force: bool) -> Fallible<ProfileId> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/publish", self.root_url, did);
        let req_fut = HttpClient::new().post(url).send_json(&force);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())));
        self.await_fut(fut)
    }

    fn restore_profile(
        &mut self,
        id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/restore", self.root_url, did);
        let req_fut = HttpClient::new().post(url).send_json(&force);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())));
        self.await_fut(fut)
    }

    fn set_attribute(
        &mut self,
        id: Option<ProfileId>,
        key: &AttributeId,
        value: &AttributeValue,
    ) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/attributes/{}", self.root_url, did, key);
        let req_fut = HttpClient::new().post(url).send_json(value);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn clear_attribute(&mut self, id: Option<ProfileId>, key: &AttributeId) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/attributes/{}", self.root_url, did, key);
        let req_fut = HttpClient::new().delete(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn claim_schemas(&self) -> Fallible<Rc<dyn ClaimSchemas>> {
        let url = format!("{}/claim-schemas", self.root_url);
        let req_fut = HttpClient::new().get(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())))
            .map(|schema_items: Vec<ClaimSchema>| {
                Rc::new(InMemoryClaimSchemas::new(schema_items)) as Rc<dyn ClaimSchemas>
            });
        self.await_fut(fut)
    }

    fn claims(&self, id: Option<ProfileId>) -> Fallible<Vec<Claim>> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/claims", self.root_url, did);
        let req_fut = HttpClient::new().get(url).send();
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| response.json().map_err(|e| SendRequestError::Body(e.into())))
            .map(|claim_items: Vec<ApiClaim>| {
                claim_items
                    .iter()
                    .filter_map(|item| {
                        // TODO we should at least log errors here
                        item.try_into().ok()
                    })
                    .collect()
            });
        self.await_fut(fut)
    }

    fn add_claim(&mut self, id: Option<ProfileId>, claim: Claim) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/claims", self.root_url, did);
        let api_claim = CreateClaim::try_from(claim)?;
        let req_fut = HttpClient::new().post(url).send_json(&api_claim);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::CREATED))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn remove_claim(&mut self, _my_profile_id: Option<ProfileId>, _claim: ClaimId) -> Fallible<()> {
        unimplemented!()
    }

    fn sign_claim(&self, id: Option<ProfileId>, claim: &SignableClaimPart) -> Fallible<ClaimProof> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/sign-claim", self.root_url, did);
        let claim_str = claim.to_string();
        debug!("Sending claim string: {}", claim_str);
        let req_fut = HttpClient::new().post(url).send_body(claim_str);
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::OK))
            .and_then(|mut response| {
                response.body().map_err(|e| {
                    warn!("Failed to fetch response body: {}", e);
                    SendRequestError::Response(ParseError::Incomplete)
                })
            })
            .and_then(|body_bytes| {
                let body_str_res = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
                    warn!("Failed to decode error message from response: {}", e);
                    SendRequestError::Response(ParseError::Utf8(e.utf8_error()))
                });
                body_str_res
            })
            .and_then(|body_str| {
                body_str.parse::<ClaimProof>().map_err(|e| SendRequestError::Body(e.into()))
            });
        self.await_fut(fut)
    }

    fn add_claim_proof(
        &mut self,
        id: Option<ProfileId>,
        claim: &ClaimId,
        proof: ClaimProof,
    ) -> Fallible<()> {
        let did = did_str(id);
        let url =
            format!("{}/vault/dids/{}/claims/{}/witness-signature", self.root_url, did, claim);
        let req_fut = HttpClient::new().put(url).send_body(proof.to_string());
        let fut = req_fut
            .and_then(|response| validate_response_status(response, StatusCode::CREATED))
            .map(|_response| ());
        self.await_fut(fut)
    }

    fn license_claim(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _claim: ClaimId,
    ) -> Fallible<ClaimLicense> {
        unimplemented!()
    }

    fn list_incoming_links(&self, _my_profile_id: Option<ProfileId>) -> Fallible<Vec<Link>> {
        unimplemented!()
    }

    fn create_link(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _peer_profile_id: &ProfileId,
    ) -> Fallible<Link> {
        unimplemented!()
    }

    fn remove_link(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _peer_profile_id: &ProfileId,
    ) -> Fallible<()> {
        unimplemented!()
    }
}

struct InMemoryClaimSchemas {
    schemas: HashMap<SchemaId, SchemaVersion>,
}

impl InMemoryClaimSchemas {
    pub fn new(schemas_vec: Vec<ClaimSchema>) -> Self {
        let mut schemas = HashMap::new();
        for schema in schemas_vec {
            let val: SchemaVersion = schema.into();
            schemas.insert(val.id().to_owned(), val);
        }
        Self { schemas }
    }
}

impl ClaimSchemas for InMemoryClaimSchemas {
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &SchemaVersion> + 'a> {
        Box::new(self.schemas.values())
    }

    fn get(&self, id: &String) -> Fallible<SchemaVersion> {
        self.schemas.get(id).cloned().ok_or_else(|| format_err!("Schema not found: {}", id))
    }
}
