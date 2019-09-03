use std::convert::{TryFrom, TryInto};

use actix_web::web;
use failure::{err_msg, Fallible};
use log::*;

use crate::data::{ProfileMetadata, *};
use crate::names::DeterministicNameGenerator;
use claims::{api::*, model::*};
use did::vault::ProfileLabel;

pub fn init_vault_impl(state: &mut Context, words: web::Json<Vec<String>>) -> Fallible<()> {
    let phrase = words.join(" ");
    state.restore_vault(phrase)?;
    state.save_vault()
}

pub fn restore_all_dids_impl(state: &mut Context) -> Fallible<RestoreCounts> {
    let counts = state.restore_all_profiles()?;
    state.save_vault()?;
    Ok(counts)
}

pub fn get_default_did_impl(state: &Context) -> Fallible<Option<ProfileId>> {
    state.get_active_profile()
}

pub fn set_default_did_impl(state: &mut Context, did_str: String) -> Fallible<()> {
    let did = did_str.parse()?;
    state.set_active_profile(&did)?;
    state.save_vault()?;
    Ok(())
}

pub fn list_dids_impl(state: &Context) -> Fallible<Vec<VaultEntry>> {
    let recs = state.list_vault_records()?;
    let entries = recs
        .iter()
        .filter_map(|record| {
            let res = VaultEntry::try_from(record);
            if res.is_err() {
                error!("Failed to convert vault record {:?} for HTTP API: {:?}", record, res);
            }
            res.ok()
        })
        .collect::<Vec<_>>();
    Ok(entries)
}

pub fn create_dids_impl(state: &mut Context, mut label: Option<String>) -> Fallible<VaultEntry> {
    let did = state.create_profile(label.clone())?;
    let did_bytes = did.to_bytes();

    if label.is_none() {
        let hd_label = DeterministicNameGenerator::default().name(&did_bytes);
        state.set_profile_label(Some(did.clone()), hd_label.clone())?;
        label = Some(hd_label);
    }

    let mut avatar_png = Vec::new();
    blockies::Ethereum::default()
        .create_icon(&mut avatar_png, &did_bytes)
        .map_err(|e| err_msg(format!("Failed to generate default profile icon: {:?}", e)))?;
    //std::fs::write(format!("/tmp/{}.png", label), &avatar_png)?;

    let mut metadata = ProfileMetadata::default();
    metadata.image_blob = avatar_png;
    metadata.image_format = "png".to_owned();
    state.set_profile_metadata(Some(did.clone()), metadata.clone().try_into()?)?;

    state.save_vault()?;
    Ok(VaultEntry {
        id: did.to_string(),
        label: label.unwrap_or_default(),
        avatar: Image { format: metadata.image_format, blob: metadata.image_blob },
        state: "TODO".to_owned(),
    })
}

fn did_opt(did_str: String) -> Fallible<Option<ProfileId>> {
    if did_str == "_" {
        return Ok(None);
    }
    let did = did_str.parse()?;
    Ok(Some(did))
}

pub fn get_did_impl(state: &Context, did_str: String) -> Fallible<VaultEntry> {
    let did = did_opt(did_str)?;
    let rec = state.get_vault_record(did)?;
    VaultEntry::try_from(&rec)
}

pub fn get_profile_impl(state: &Context, did_str: String) -> Fallible<PrivateProfileData> {
    let did = did_opt(did_str)?;
    state.get_profile_data(did, ProfileRepositoryKind::Local)
}

pub fn restore_did_impl(
    state: &mut Context,
    did_str: String,
    force: bool,
) -> Fallible<PrivateProfileData> {
    let did = did_opt(did_str)?;
    state.restore_profile(did, force)
}

pub fn revert_did_impl(state: &mut Context, did_str: String) -> Fallible<PrivateProfileData> {
    let did = did_opt(did_str)?;
    state.revert_profile(did)
}

pub fn publish_did_impl(state: &mut Context, did_str: String, force: bool) -> Fallible<ProfileId> {
    let did = did_opt(did_str)?;
    state.publish_profile(did, force)
}

pub fn rename_did_impl(
    state: &mut Context,
    did_str: String,
    //did: ProfileId,
    name: ProfileLabel,
) -> Fallible<()> {
    let did = did_opt(did_str)?;
    state.set_profile_label(did, name)?;
    state.save_vault()?;
    Ok(())
}

pub fn set_avatar_impl(
    state: &mut Context,
    did_str: String,
    avatar_datauri: DataUri,
) -> Fallible<()> {
    let did = did_opt(did_str)?;
    let (format, avatar_binary) = parse_avatar(&avatar_datauri)?;
    let metadata_ser = state.get_profile_metadata(did.clone())?;
    let mut metadata = ProfileMetadata::try_from(metadata_ser.as_str())?;
    metadata.image_format = format;
    metadata.image_blob = avatar_binary;
    state.set_profile_metadata(did, metadata.try_into()?)?;
    state.save_vault()?;
    Ok(())
}

pub fn set_did_attribute_impl(
    state: &mut Context,
    path: AttributePath,
    val: AttributeValue,
) -> Fallible<()> {
    let did = did_opt(path.did)?;
    state.set_attribute(did, &path.attribute_id, &val)
}

pub fn clear_did_attribute_impl(state: &mut Context, path: AttributePath) -> Fallible<()> {
    let did = did_opt(path.did)?;
    state.clear_attribute(did, &path.attribute_id)
}

pub fn list_did_claims_impl(state: &Context, did_str: String) -> Fallible<Vec<ApiClaim>> {
    let did = did_opt(did_str)?;
    let claims = state.claims(did.clone())?;

    let rec = state.get_vault_record(did)?;
    let schema_registry = state.claim_schemas()?;
    let claims = claims
        .iter()
        .filter_map(|claim| {
            let res = ApiClaim::try_from(claim, rec.label(), &*schema_registry);
            if res.is_err() {
                error!("Failed to convert claim {:?} for HTTP API: {:?}", claim, res);
            }
            res.ok()
        })
        .collect();
    Ok(claims)
}

pub fn list_vault_claims_impl(state: &Context) -> Fallible<Vec<ApiClaim>> {
    let schema_registry = state.claim_schemas()?;

    let mut claims = Vec::new();
    for rec in state.list_vault_records()? {
        let did_claims = state.claims(Some(rec.id()))?;
        for claim in did_claims {
            claims.push(ApiClaim::try_from(&claim, rec.label(), &*schema_registry)?);
        }
    }

    Ok(claims)
}

pub fn create_did_claim_impl(
    state: &mut Context,
    did_str: String,
    claim_details: CreateClaim,
) -> Fallible<ContentId> {
    let did = did_opt(did_str)?.or(state.get_active_profile()?);

    let subject = did
        .clone()
        .ok_or_else(|| err_msg("No profile specified and no active profile set in vault"))?;
    let claim =
        Claim::new(subject, claim_details.schema, serde_json::to_vec(&claim_details.content)?);
    let claim_id = claim.id();

    state.add_claim(did, claim)?;
    state.save_vault()?;
    Ok(claim_id)
}

pub fn delete_claim_impl(state: &mut Context, claim_path: ClaimPath) -> Fallible<()> {
    let did = did_opt(claim_path.did)?;
    let claim_id = claim_path.claim_id;
    state.remove_claim(did, claim_id)?;
    state.save_vault()?;
    Ok(())
}

pub fn list_schemas_impl(state: &Context) -> Fallible<Vec<ClaimSchema>> {
    let repo = state.claim_schemas()?;
    Ok(repo.iter().map(|v| v.into()).collect::<Vec<_>>())
}
