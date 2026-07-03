use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthenticatedApiKey {
    pub id: Uuid,
    pub rate_limit_per_minute: i32,
}
