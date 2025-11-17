pub mod comparer;
pub mod enrollment;
pub mod extractor;
pub mod store;

pub use comparer::{
    cosine_similarity, run_face_comparison, FaceComparisonConfig, FaceComparisonOutcome,
    FaceComparisonScore,
};

pub use enrollment::{
    map_to_embedding_validation, run_face_enrollment, run_face_enrollment_with, run_face_removal,
    run_face_removal_with, validate_user_name, EnrollmentRecord, FaceEnrollmentConfig,
    FaceEnrollmentOutcome, FaceRemovalConfig, FaceRemovalOutcome, KeyProvider,
    SecretServiceKeyProvider,
};

pub use extractor::{
    ensure_valid_faces, run_face_extraction, run_face_extraction_with_backend, BoundingBox,
    DlibBackend, EnvModelPathResolver, FaceEmbeddingBackend, FaceEmbeddingRecord,
    FaceExtractionConfig, FaceExtractionOutcome, FaceExtractionSummary, FaceModelPaths,
    ModelPathResolver,
};

pub use store::{
    load_enrolled_embeddings, user_store_path, EnrolledEmbedding, EnvStoreDirResolver, FaceStore,
    FilesystemFaceStore, StoreDirResolver,
};
