use crate::auth::bearer::BearerAuthList;
use crate::auth::jwks::JwksAuthList;

// Since we have multiple bearer auth types, including:
// - Bearer
// - JWKS
// this file is to handle looping over both
