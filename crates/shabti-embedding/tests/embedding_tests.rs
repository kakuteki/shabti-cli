use shabti_core::traits::EmbeddingModel;
use shabti_embedding::FastEmbedModel;

#[test]
fn model_id_returns_expected_name() {
    let model = FastEmbedModel::new().unwrap();
    assert!(!model.model_id().is_empty());
    assert!(
        model.model_id().contains("Multilingual") || model.model_id().contains("E5"),
        "unexpected model_id: {}",
        model.model_id()
    );
}

#[test]
fn dimension_is_384() {
    let model = FastEmbedModel::new().unwrap();
    assert_eq!(model.dimension(), 384);
}

#[test]
fn embed_returns_correct_dimension() {
    let model = FastEmbedModel::new().unwrap();
    let embedding = model.embed("Hello, world!").unwrap();
    assert_eq!(embedding.len(), 384);
}

#[test]
fn embed_returns_non_zero_vector() {
    let model = FastEmbedModel::new().unwrap();
    let embedding = model.embed("This is a test").unwrap();
    let sum: f32 = embedding.iter().map(|v| v.abs()).sum();
    assert!(sum > 0.0);
}

#[test]
fn embed_batch_returns_correct_count() {
    let model = FastEmbedModel::new().unwrap();
    let texts = &["Hello", "World", "Test"];
    let embeddings = model.embed_batch(texts).unwrap();
    assert_eq!(embeddings.len(), 3);
    for emb in &embeddings {
        assert_eq!(emb.len(), 384);
    }
}

#[test]
fn similar_texts_have_higher_similarity() {
    let model = FastEmbedModel::new().unwrap();
    let emb_cat = model.embed("I love cats").unwrap();
    let emb_kitten = model.embed("I adore kittens").unwrap();
    let emb_car = model.embed("The car engine needs repair").unwrap();

    let sim_cat_kitten = cosine_similarity(&emb_cat, &emb_kitten);
    let sim_cat_car = cosine_similarity(&emb_cat, &emb_car);

    assert!(
        sim_cat_kitten > sim_cat_car,
        "cat-kitten similarity ({sim_cat_kitten}) should be > cat-car similarity ({sim_cat_car})"
    );
}

#[test]
fn embed_empty_string_does_not_panic() {
    let model = FastEmbedModel::new().unwrap();
    let embedding = model.embed("").unwrap();
    assert_eq!(embedding.len(), 384);
}

#[test]
fn embed_japanese_text() {
    let model = FastEmbedModel::new().unwrap();
    let emb_ja1 = model.embed("猫が好きです").unwrap();
    let emb_ja2 = model.embed("犬が好きです").unwrap();
    let emb_unrelated = model.embed("経済政策の影響").unwrap();

    let sim_animals = cosine_similarity(&emb_ja1, &emb_ja2);
    let sim_unrelated = cosine_similarity(&emb_ja1, &emb_unrelated);

    assert!(
        sim_animals > sim_unrelated,
        "animal similarity ({sim_animals}) should be > unrelated ({sim_unrelated})"
    );
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
