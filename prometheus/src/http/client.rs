use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;
use std::str::FromStr;

use actix_web::client::Client as HttpClient;
use failure::{err_msg, format_err, Fallible};
use futures::Future;
//use log::*;

use crate::data::*;
use claims::api::*;
use claims::model::*;
use did::vault::{ProfileLabel, ProfileMetadata, ProfileVaultRecord};

pub struct ApiHttpClient {
    root_url: String,
    reactor: RefCell<actix_rt::SystemRunner>,
}

impl ApiHttpClient {
    pub fn new(root_url: &str) -> Self {
        Self {
            root_url: root_url.to_owned(),
            reactor: RefCell::new(actix_rt::System::new("ActixReactor")),
        }
    }

    fn await_fut<T>(
        &self,
        fut: impl Future<Item = T, Error = actix_web::error::Error>,
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

impl Api for ApiHttpClient {
    fn restore_vault(&mut self, phrase: String) -> Fallible<()> {
        let url = format!("{}/vault", self.root_url);
        // TODO phrase should normally be splitted into words and sent that way,
        //      but this will work for the moment
        let req_fut = HttpClient::new().post(url).send_json(&vec![phrase]).from_err();
        let fut = req_fut.and_then(|mut response| {
            // TODO this probably ignores status code, so we should check it properly
            response.body().from_err().and_then(|_body| {
                //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
                Ok(())
            })
        });
        self.await_fut(fut)
    }

    fn restore_all_profiles(&mut self) -> Fallible<RestoreCounts> {
        let url = format!("{}/vault/restore-dids", self.root_url);
        let req_fut = HttpClient::new().post(url).send().from_err();
        let fut = req_fut.and_then(|mut response| {
            // TODO this probably ignores status code, so we should check it properly
            response.body().from_err().and_then(|body| {
                //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
                let counts = serde_json::from_slice(&body)?;
                Ok(counts)
            })
        });
        self.await_fut(fut)
    }

    fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()> {
        let url = format!("{}/vault/default-did", self.root_url);
        let req_fut = HttpClient::new().put(url).send_json(&my_profile_id.to_string()).from_err();
        let fut = req_fut.and_then(|mut response| {
            // TODO this probably ignores status code, so we should check it properly
            response.body().from_err().and_then(|_body| {
                //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
                Ok(())
            })
        });
        self.await_fut(fut)
    }

    fn get_active_profile(&self) -> Fallible<Option<ProfileId>> {
        let url = format!("{}/vault/default-did", self.root_url);
        let req_fut = HttpClient::new().get(url).send().from_err();
        let fut = req_fut.and_then(|mut response| {
            // TODO this probably ignores status code, so we should check it properly
            response.body().from_err().and_then(|body| {
                // info!("Received response: {:?}", String::from_utf8(body.to_vec()).unwrap_or_default() );
                let active_opt = match serde_json::from_slice::<Option<String>>(&body)? {
                    None => None,
                    Some(did_str) => Some(did_str.parse()?),
                };
                Ok(active_opt)
            })
        });
        self.await_fut(fut)
    }

    fn list_vault_records(&self) -> Fallible<Vec<ProfileVaultRecord>> {
        let url = format!("{}/vault/dids", self.root_url);
        let req_fut = HttpClient::new().get(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let entries: Vec<VaultEntry> = serde_json::from_slice(&body)?;
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

    fn create_profile(&mut self, label: Option<ProfileLabel>) -> Fallible<ProfileId> {
        let url = format!("{}/vault/dids", self.root_url);
        let req_fut = HttpClient::new().post(url).send_json(&label).from_err();
        let fut = req_fut.and_then(|mut response| {
            // TODO this probably ignores status code, so we should check it properly
            response.body().from_err().and_then(|body| {
                //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
                let entry: VaultEntry = serde_json::from_slice(&body)?;
                let id = ProfileId::from_str(&entry.id)?;
                Ok(id)
            })
        });
        self.await_fut(fut)
    }

    fn get_vault_record(&self, id: Option<ProfileId>) -> Fallible<ProfileVaultRecord> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}", self.root_url, did);
        let req_fut = HttpClient::new().get(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let entry: VaultEntry = serde_json::from_slice(&body)?;
            let rec = (&entry).try_into()?;
            Ok(rec)
        });
        self.await_fut(fut)
    }

    fn set_profile_label(&mut self, id: Option<ProfileId>, label: ProfileLabel) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/label", self.root_url, did);
        let req_fut = HttpClient::new().put(url).send_json(&label).from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|_body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            Ok(())
        });
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
        let req_fut = HttpClient::new().get(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let prof: PrivateProfileData = serde_json::from_slice(&body)?;
            Ok(prof)
        });
        self.await_fut(fut)
    }

    fn revert_profile(&mut self, id: Option<ProfileId>) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/revert", self.root_url, did);
        let req_fut = HttpClient::new().post(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let prof: PrivateProfileData = serde_json::from_slice(&body)?;
            Ok(prof)
        });
        self.await_fut(fut)
    }

    fn publish_profile(&mut self, id: Option<ProfileId>, force: bool) -> Fallible<ProfileId> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/publish", self.root_url, did);
        let req_fut = HttpClient::new().post(url).send_json(&force).from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let id: ProfileId = serde_json::from_slice(&body)?;
            Ok(id)
        });
        self.await_fut(fut)
    }

    fn restore_profile(
        &mut self,
        id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/restore", self.root_url, did);
        let req_fut = HttpClient::new().post(url).send_json(&force).from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let prof: PrivateProfileData = serde_json::from_slice(&body)?;
            Ok(prof)
        });
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
        let req_fut = HttpClient::new().post(url).send_json(&value).from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|_body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            Ok(())
        });
        self.await_fut(fut)
    }

    fn clear_attribute(&mut self, id: Option<ProfileId>, key: &AttributeId) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/attributes/{}", self.root_url, did, key);
        let req_fut = HttpClient::new().delete(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|_body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            Ok(())
        });
        self.await_fut(fut)
    }

    fn claim_schemas(&self) -> Fallible<Rc<dyn ClaimSchemas>> {
        let url = format!("{}/claim-schemas", self.root_url);
        let req_fut = HttpClient::new().get(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let schema_items: Vec<ClaimSchema> = serde_json::from_slice(&body)?;
            let schemas = Rc::new(InMemoryClaimSchemas::new(schema_items)) as Rc<dyn ClaimSchemas>;
            Ok(schemas)
        });
        self.await_fut(fut)
    }

    fn claims(&self, id: Option<ProfileId>) -> Fallible<Vec<Claim>> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/claims", self.root_url, did);
        let req_fut = HttpClient::new().get(url).send().from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            let claim_items: Vec<ApiClaim> = serde_json::from_slice(&body)?;
            let claims = claim_items
                .iter()
                .filter_map(|item| {
                    // TODO we should at least log errors here
                    item.try_into().ok()
                })
                .collect();
            Ok(claims)
        });
        self.await_fut(fut)
    }

    fn add_claim(&mut self, id: Option<ProfileId>, claim: Claim) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/claims", self.root_url, did);
        let api_claim = CreateClaim::try_from(claim)?;
        let req_fut = HttpClient::new().post(url).send_json(&api_claim).from_err();
        // TODO this probably ignores status code, so we should check it properly
        let fut = req_fut.and_then(|mut response| response.body().from_err()).and_then(|_body| {
            //info!("Received response: {:?}", String::from_utf8(body.to_vec()));
            Ok(())
        });
        self.await_fut(fut)
    }

    fn remove_claim(&mut self, _my_profile_id: Option<ProfileId>, _claim: ClaimId) -> Fallible<()> {
        unimplemented!()
    }

    fn add_claim_proof(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _claim: ClaimId,
        _proof: ClaimProof,
    ) -> Fallible<()> {
        unimplemented!()
    }

    fn present_claim(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _claim: ClaimId,
    ) -> Fallible<ClaimPresentation> {
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
