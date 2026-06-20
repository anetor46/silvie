DROP INDEX IF EXISTS travel_documents_user_type_primary_unique;
DROP INDEX IF EXISTS travel_documents_user_expiry_idx;
DROP INDEX IF EXISTS travel_documents_user_type_idx;
DROP TABLE IF EXISTS travel_documents;

DROP INDEX IF EXISTS addresses_user_type_unique;
DROP INDEX IF EXISTS addresses_user_type_idx;
DROP TABLE IF EXISTS addresses;

DROP TABLE IF EXISTS user_profiles;
