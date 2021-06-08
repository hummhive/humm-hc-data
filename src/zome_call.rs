use hdk::prelude::*;
use hdk::hash_path::path::Component;

#[derive(thiserror::Error)]
enum Error {
    #[error("Attempted to delete an entry")]
    DeleteAttempted,
    #[error("Attempted to update an entry")]
    UpdateAttempted,
}

impl From<Error> for ExternResult<ValidateCallbackResult> {
    fn from(e: Error) -> Self {
        Ok(ValidateCallbackResult::Invalid(e.to_string()))
    }
}

type Squuid = String;
// Sha512 + file extension.
type BlobId = String;
type RevisionId = String;
type Sha512 = String;
type Mime = String;
type Json = String;

#[hdk_entry(id = "blob")]
struct Blob {
    sha512: Sha512,
    blob: Bytes,
    mimetype: Mime,
}

#[hdk_entry(id = "revision")]
struct Revision {
    squuid: RevisionId,
    data: Json,
}

struct RevisionData {
    data: Vec<(DataId, Vec<Revision>)>,
    blobs: Vec<Blob>,
}

struct RevisionDigest {
    data: Vec<(DataId, Vec<RevisionId>)>,
    blobs: Vec<BlobId>,
}

#[hdk_extern]
fn validate_create_entry_revision(validate_data: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Revision::try_from(&validate_data.element)?;
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
fn validate_create_entry_blob(validate_data: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Blob::try_from(&validate_data.element)?;
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
fn validate_delete(_: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Error::DeleteAttempted.into()
}

#[hdk_extern]
fn validate_update(_: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Error::UpdateAttempted.into()
}

#[hdk_extern]
fn get_revision_digest(hive_id: String) -> ExternResult<RevisionDigest> {
    let mut revision_digest = RevisionDigest {
        data: vec![],
        blobs: vec![],
    };

    let data_links: Vec<Link> = Path::from(format!("{}.data", hive_id)).children()?.into_inner();
    for data_link in data_links.iter() {
        let components: Vec<Component> = Path::from(data_link.tag).into();
        let data_id = components.pop()?;

        let mut revision_ids: Vec<RevisionId> = vec![];
        let revision_id_links: Vec<Link> = Path::from(format!("{}.data.{}", hive_id, data_id)).children()?.into_inner();
        for revision_id_link in revision_id_links.iter() {
            let components: Vec<Component> = Path::from(revision_id_link.tag).into();
            revision_ids.push(components.pop()?);
        }

        revision_digest.data.push((data_id, revision_ids));
    }

    let blob_links: Vec<Link> = Path::from(format!("{}.blob.{}", hive_id)).children()?.into_inner();
    for blob_link in blob_links.iter() {
        let components: Vec<Component> = Path::from(blob_link.tag).into();
        revision_digest.blobs.push(components.pop()?);
    }

    Ok(revision_digest)
}

#[hdk_extern]
fn get_revision_data(hive_id: String, revision_digest: RevisionDigest) -> ExternResult<RevisionData> {
    let mut revision_data = RevisionData {
        data: vec![],
        blobs: vec![],
    };
    for (data_id, revision_ids) in revision_digest.data.iter() {
        let mut revisions: Vec<Revision> = vec![];

        for revision_id in revision_ids.iter() {

            let children: Vec<Link> = Path::from(format!("{}.data.{}.{}", hive_id, data_id, revision_id)).children()?.into_inner();
            for child in children.iter() {
                let components: Vec<Component> = Path::from(child.tag).into();
                let entry_hash = EntryHash::from_39_bytes(hex::decode(components.pop()?));

                let revision = Revision::try_from(must_get_entry(entry_hash)?)?;
                revisions.push(revision);
            }
        }

        revision_data.data.push((data_id, revisions));
    }

    for blob_id in revision_digest.blobs.iter() {
        let children: Vec<Link> = Path::from(format!("{}.blob.{}", hive_id, blob_id)).children()?.into_inner();
        for child in children.iter() {
            let components: Vec<Component> = Path::from(child.tag).into();
            let entry_hash = EntryHash::from_39_bytes(hex::decode(components.pop()?));

            let blob = Blob::try_from(must_get_entry(entry_hash)?)?;
            revision_data.blobs.push(blob);
        }
    }

    Ok(revision_data)
}

#[hdk_extern]
fn set_revision_data(hive_id: String, revision_data: RevisionData) -> ExternResult<()> {
    for (data_id, revisions) in revision_data.data.iter() {
        for revision in revisions.iter() {
            create_entry(revision)?;

            Path::from(format!("{}.data.{}.{}.{}", hive_id, data_id, revision.squuid, hex!(hash_entry(revision)?))).ensure()?;
        }
    }

    for blob in revision_data.blobs.iter() {
        create_entry(blob)?;

        Path::from(format!("{}.blob.{}.{}", hive_id, blob.sha512, hex!(hash_entry(blob)?))).ensure()?;
    }

    Ok(())
}