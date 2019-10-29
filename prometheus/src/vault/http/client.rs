use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

use actix_http::http::StatusCode;
use actix_web::client::{Client as HttpClient, ClientResponse};
use async_trait::async_trait;
use failure::{format_err, Fallible};
use futures::compat::Future01CompatExt;
use multiaddr::Multiaddr;
//use log::*;

use crate::daemon::NetworkState;
use crate::*;
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
}

fn did_str(did_opt: Option<ProfileId>) -> String {
    match did_opt {
        None => "_".to_owned(),
        Some(did) => did.to_string(),
    }
}

type AwcResponse =
    ClientResponse<actix_http::encoding::Decoder<actix_http::Payload<actix_http::PayloadStream>>>;

pub struct CallResponse(AwcResponse);

impl CallResponse {
    // TODO we should also log and return more response contents describing more error details
    // This method is only async to gather the error message from the response
    pub async fn check_status(mut self, status: StatusCode) -> Fallible<ValidatedCallResponse> {
        if self.0.status() == status {
            return Ok(ValidatedCallResponse(self.0));
        }

        warn!("Got response with unexpected status: {}", self.0.status());
        let body = ValidatedCallResponse::get_body(&mut self.0).await?;

        let body_str = String::from_utf8(body.to_vec())
            .map_err(|e| format_err!("Failed to decode error message from response: {}", e))?;
        Err(format_err!("Error details: {}", body_str))
    }
}

pub struct ValidatedCallResponse(AwcResponse);

impl ValidatedCallResponse {
    pub async fn ignore(mut self) -> Fallible<()> {
        let _body = Self::get_body(&mut self.0).await?;
        Ok(())
    }

    pub async fn json<V: serde::de::DeserializeOwned>(mut self) -> Fallible<V> {
        let json: V = self
            .0
            .json()
            .compat()
            .await
            .map_err(|e| format_err!("Failed to parse response as JSON: {}", e))?;
        Ok(json)
    }

    pub async fn body(mut self) -> Fallible<bytes::Bytes> {
        Self::get_body(&mut self.0).await
    }

    async fn get_body(response: &mut AwcResponse) -> Fallible<bytes::Bytes> {
        let bytes = response
            .body()
            .compat()
            .await
            .map_err(|e| format_err!("Failed to gather error message from response: {}", e))?;
        Ok(bytes)
    }
}

async fn call<FReq>(req: FReq) -> Fallible<CallResponse>
where
    FReq: FnOnce(HttpClient) -> awc::SendClientRequest,
{
    let request = req(HttpClient::new());
    let response =
        request.compat().await.map_err(|e| format_err!("Error sending request: {}", e))?;
    Ok(CallResponse(response))
}

#[async_trait(?Send)]
impl VaultApi for VaultClient {
    async fn restore_vault(&mut self, phrase: String) -> Fallible<()> {
        let url = format!("{}/vault", self.root_url);
        // TODO phrase should normally be splitted into words and sent that way,
        //      but this will work for the moment
        let response = call(move |c| c.post(url).send_json(&vec![phrase])).await?;
        let validated = response.check_status(StatusCode::CREATED).await?;
        let result = validated.ignore().await?;
        Ok(result)
    }

    async fn restore_all_profiles(&mut self) -> Fallible<RestoreCounts> {
        let url = format!("{}/vault/restore-dids", self.root_url);
        let response = call(move |c| c.post(url).send()).await?;
        let validated = response.check_status(StatusCode::CREATED).await?;
        let result: RestoreCounts = validated.json().await?;
        Ok(result)
    }

    async fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()> {
        let url = format!("{}/vault/default-did", self.root_url);
        let response = call(move |c| c.put(url).send_json(&my_profile_id.to_string())).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.ignore().await?;
        Ok(result)
    }

    async fn get_active_profile(&self) -> Fallible<Option<ProfileId>> {
        let url = format!("{}/vault/default-did", self.root_url);
        let response = call(move |c| c.get(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let did_str_opt: Option<String> = validated.json().await?;
        let result = match did_str_opt {
            None => None,
            Some(did_str) => Some(did_str.parse()?),
        };
        Ok(result)
    }

    //async fn list_vault_records(&self) -> Fallible<Vec<VaultEntry>> {
    async fn list_vault_records(&self) -> Fallible<Vec<ProfileVaultRecord>> {
        let url = format!("{}/vault/dids", self.root_url);
        let response = call(move |c| c.get(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let entries: Vec<VaultEntry> = validated.json().await?;
        let result = entries
            .iter()
            .filter_map(|entry| {
                // TODO we should at least log errors here
                entry.try_into().ok()
            })
            .collect();
        Ok(result)
    }

    async fn create_profile(
        &mut self,
        label: Option<ProfileLabel>,
    ) -> Fallible<ProfileVaultRecord> {
        let url = format!("{}/vault/dids", self.root_url);
        let response = call(move |c| c.get(url).send_json(&label.unwrap_or_default())).await?;
        let validated = response.check_status(StatusCode::CREATED).await?;
        let entry: VaultEntry = validated.json().await?;
        let result = (&entry).try_into()?;
        Ok(result)
    }

    async fn get_vault_record(&self, id: Option<ProfileId>) -> Fallible<ProfileVaultRecord> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}", self.root_url, did);
        let response = call(move |c| c.get(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let entry: VaultEntry = validated.json().await?;
        let result = (&entry).try_into()?;
        Ok(result)
    }

    async fn set_profile_label(
        &mut self,
        id: Option<ProfileId>,
        label: ProfileLabel,
    ) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/label", self.root_url, did);
        let response = call(move |c| c.get(url).send_json(&label)).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.ignore().await?;
        Ok(result)
    }

    async fn get_profile_metadata(&self, _id: Option<ProfileId>) -> Fallible<ProfileMetadata> {
        unimplemented!() // NOTE not present in the CLI so far
    }

    async fn set_profile_metadata(
        &mut self,
        _id: Option<ProfileId>,
        _data: ProfileMetadata,
    ) -> Fallible<()> {
        unimplemented!() // NOTE not present in the CLI so far
    }

    async fn get_profile_data(
        &self,
        id: Option<ProfileId>,
        _repo_kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/profiledata", self.root_url, did);
        let response = call(move |c| c.get(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.json().await?;
        Ok(result)
    }

    async fn revert_profile(&mut self, id: Option<ProfileId>) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/revert", self.root_url, did);
        let response = call(move |c| c.post(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.json().await?;
        Ok(result)
    }

    async fn publish_profile(&mut self, id: Option<ProfileId>, force: bool) -> Fallible<ProfileId> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/publish", self.root_url, did);
        let response = call(move |c| c.post(url).send_json(&force)).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.json().await?;
        Ok(result)
    }

    async fn restore_profile(
        &mut self,
        id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/restore", self.root_url, did);
        let response = call(move |c| c.post(url).send_json(&force)).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.json().await?;
        Ok(result)
    }

    async fn set_attribute(
        &mut self,
        id: Option<ProfileId>,
        key: &AttributeId,
        value: &AttributeValue,
    ) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/attributes/{}", self.root_url, did, key);
        let response = call(move |c| c.post(url).send_json(value)).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.ignore().await?;
        Ok(result)
    }

    async fn clear_attribute(&mut self, id: Option<ProfileId>, key: &AttributeId) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/attributes/{}", self.root_url, did, key);
        let response = call(move |c| c.delete(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let result = validated.json().await?;
        Ok(result)
    }

    async fn claims(&self, id: Option<ProfileId>) -> Fallible<Vec<Claim>> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/claims", self.root_url, did);
        let response = call(move |c| c.get(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let claim_items: Vec<ApiClaim> = validated.json().await?;
        let result = claim_items
            .iter()
            .filter_map(|item| {
                // TODO we should at least log errors here
                item.try_into().ok()
            })
            .collect();
        Ok(result)
    }

    async fn add_claim(&mut self, id: Option<ProfileId>, claim: Claim) -> Fallible<()> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/claims", self.root_url, did);
        let api_claim = CreateClaim::try_from(claim)?;
        let response = call(move |c| c.post(url).send_json(&api_claim)).await?;
        let validated = response.check_status(StatusCode::CREATED).await?;
        let result = validated.ignore().await?;
        Ok(result)
    }

    async fn remove_claim(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _claim: ClaimId,
    ) -> Fallible<()> {
        unimplemented!()
    }

    async fn sign_claim(
        &self,
        id: Option<ProfileId>,
        claim: &SignableClaimPart,
    ) -> Fallible<ClaimProof> {
        let did = did_str(id);
        let url = format!("{}/vault/dids/{}/sign-claim", self.root_url, did);
        let claim_str = claim.to_string();
        debug!("Sending claim string: {}", claim_str);
        let response = call(move |c| c.post(url).send_body(claim_str)).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let claim_bytes = validated.body().await?;
        let claim_str = String::from_utf8(claim_bytes.to_vec())?;
        let result: ClaimProof = claim_str.parse()?;
        Ok(result)
    }

    async fn add_claim_proof(
        &mut self,
        id: Option<ProfileId>,
        claim: &ClaimId,
        proof: ClaimProof,
    ) -> Fallible<()> {
        let did = did_str(id);
        let url =
            format!("{}/vault/dids/{}/claims/{}/witness-signature", self.root_url, did, claim);
        let response = call(move |c| c.put(url).send_body(proof.to_string())).await?;
        let validated = response.check_status(StatusCode::CREATED).await?;
        let result = validated.ignore().await?;
        Ok(result)
    }

    async fn license_claim(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _claim: ClaimId,
    ) -> Fallible<ClaimLicense> {
        unimplemented!()
    }

    //async fn list_incoming_links(&self, _my_profile_id: Option<ProfileId>) -> Fallible<Vec<Link>> {
    //    unimplemented!()
    // NOTE this has to consult an explorer, not the Vault
    // let profile = self.selected_profile(my_profile_id)?;
    // let followers = self.explorer.followers(&profile.id()).wait()?;
    // Ok(followers)
    //}

    async fn create_link(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _peer_profile_id: &ProfileId,
    ) -> Fallible<Link> {
        unimplemented!()
    }

    async fn remove_link(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _peer_profile_id: &ProfileId,
    ) -> Fallible<()> {
        unimplemented!()
    }

    async fn did_homes(&self, _my_profile_id: Option<ProfileId>) -> Fallible<Vec<DidHomeStatus>> {
        unimplemented!()
    }

    async fn register_home<'a, 'b>(
        &'a mut self,
        _my_id: Option<ProfileId>,
        _home_id: &'b ProfileId,
        _addr_hints: &'b [Multiaddr],
        _network: &'a mut NetworkState,
    ) -> Fallible<()> {
        unimplemented!()
    }

    async fn claim_schemas(&self) -> Fallible<Rc<dyn ClaimSchemas>> {
        let url = format!("{}/claim-schemas", self.root_url);
        let response = call(move |c| c.get(url).send()).await?;
        let validated = response.check_status(StatusCode::OK).await?;
        let schema_items: Vec<ClaimSchema> = validated.json().await?;
        let result: Rc<dyn ClaimSchemas> = Rc::new(InMemoryClaimSchemas::new(schema_items));
        Ok(result)
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
