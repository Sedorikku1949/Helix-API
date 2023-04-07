use super::cdn::CdnId;

pub struct Channel {
    id: String,
    name: String,
    owner: String,
    icon: CdnId
}