use hdk::prelude::*;
use hdk::hash_path::path::Component;

#[derive(Debug, thiserror::Error)]
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

type HiveId = String;
// Sha512 + file extension.
type BlobId = String;
type RevisionId = String;
type Sha512 = String;
type Mime = String;
type Json = String;
type DataId = String;

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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RevisionData {
    data: Vec<(DataId, Vec<Revision>)>,
    blobs: Vec<Blob>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
fn get_revision_digest(hive_id: HiveId) -> ExternResult<RevisionDigest> {
    let mut revision_digest = RevisionDigest {
        data: vec![],
        blobs: vec![],
    };

    let data_links: Vec<Link> = Path::from(format!("{}.data", hive_id)).children()?.into_inner();
    for data_link in data_links.iter() {
        match <Vec<Component>>::from(Path::try_from(&data_link.tag)?).pop() {
            Some(data_id_component) => {
                let data_id = DataId::try_from(&data_id_component)?;
                let mut revision_ids: Vec<RevisionId> = vec![];
                let revision_id_links: Vec<Link> = Path::from(format!("{}.data.{}", hive_id, data_id)).children()?.into_inner();
                for revision_id_link in revision_id_links.iter() {
                    match <Vec<Component>>::from(Path::try_from(&revision_id_link.tag)?).pop() {
                        Some(revision_id_component) => {
                            revision_ids.push(RevisionId::try_from(&revision_id_component)?);
                        },
                        None => { },
                    }
                }

                revision_digest.data.push((data_id, revision_ids));
            },
            None => { },
        }


    }

    let blob_links: Vec<Link> = Path::from(format!("{}.blob", hive_id)).children()?.into_inner();
    for blob_link in blob_links.iter() {
        match <Vec<Component>>::from(Path::try_from(&blob_link.tag)?).pop() {
            Some(last_component) => {
                revision_digest.blobs.push(Sha512::try_from(&last_component)?);
            },
            None => { },
        }
    }

    Ok(revision_digest)
}

type GetRevisionDataInput = (HiveId, RevisionDigest);

#[hdk_extern]
fn get_revision_data(input: GetRevisionDataInput) -> ExternResult<RevisionData> {
    let (hive_id, revision_digest) = input;
    let mut revision_data = RevisionData {
        data: vec![],
        blobs: vec![],
    };
    for (data_id, revision_ids) in revision_digest.data.iter() {
        let mut revisions: Vec<Revision> = vec![];

        for revision_id in revision_ids.iter() {

            let children: Vec<Link> = Path::from(format!("{}.data.{}.{}", hive_id, data_id, revision_id)).children()?.into_inner();
            for child in children.iter() {
                // @todo correct to bail if child tag does not turn into path?
                match <Vec<Component>>::from(Path::try_from(&child.tag)?).pop() {
                    Some(last_component) => {
                        let entry_hash = EntryHash::from_raw_32(last_component.into());
                        // @todo is it correct to bail on missing entry?
                        let revision = Revision::try_from(must_get_entry(entry_hash)?)?;
                        revisions.push(revision);
                    },
                    // @todo is this right?
                    None => { },
                }
            }
        }

        revision_data.data.push((data_id.clone(), revisions));
    }

    for blob_id in revision_digest.blobs.iter() {
        let children: Vec<Link> = Path::from(format!("{}.blob.{}", hive_id, blob_id)).children()?.into_inner();
        for child in children.iter() {
            // @todo correct to bail if child tag does not turn into path?
            match <Vec<Component>>::from(Path::try_from(&child.tag)?).pop() {
                Some(last_component) => {
                    let entry_hash = EntryHash::from_raw_32(last_component.into());
                    // @todo is it correct to bail on missing entry?
                    let blob = Blob::try_from(must_get_entry(entry_hash)?)?;
                    revision_data.blobs.push(blob);
                },
                // @todo is this right?
                None => { },
            }
        }
    }

    Ok(revision_data)
}

type SetRevisionDataInput = (HiveId, RevisionData);

#[hdk_extern]
fn set_revision_data(input: SetRevisionDataInput) -> ExternResult<()> {
    let (hive_id, revision_data) = input;
    for (data_id, revisions) in revision_data.data.iter() {
        for revision in revisions.iter() {
            create_entry(revision)?;

            let mut components: Vec<Component> = Path::from(format!("{}.data.{}.{}", hive_id, data_id, revision.squuid)).into();
            components.push(hash_entry(revision)?.get_raw_32().to_vec().into());
            Path::from(components).ensure()?;
        }
    }

    for blob in revision_data.blobs.iter() {
        create_entry(blob)?;

        let mut components: Vec<Component> = Path::from(format!("{}.blob.{}", hive_id, blob.sha512)).into();
        components.push(hash_entry(blob)?.get_raw_32().to_vec().into());
        Path::from(components).ensure()?;
    }

    Ok(())
}