//! Planificateur de budget mémoire : répartit la VRAM entre poids du modèle,
//! cache KV, tampons de calcul et autres modèles résidents, puis déduit les
//! paramètres de `llama-server` (couches sur GPU, contexte, quantification du KV).

use crate::config::{HardwareConfig, LlmConfig};
use serde::Serialize;

const COMPUTE_BUFFER_GB: f64 = 0.8;
const GB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LlmPlan {
    pub n_gpu_layers: u32,
    pub ctx_tokens: u32,
    /// Quantification du cache KV : "f16" ou "q8_0".
    pub kv_type: String,
    pub weights_gb: f64,
    pub kv_gb: f64,
    pub vram_used_gb: f64,
    pub vram_budget_gb: f64,
    pub ram_used_gb: f64,
    pub full_offload: bool,
    pub notes: Vec<String>,
}

fn kv_gb(llm: &LlmConfig, ctx: u32, bytes_per_elem: f64) -> f64 {
    // K et V : 2 × couches × têtes KV × dim × contexte × octets
    2.0 * llm.n_layers as f64 * llm.n_kv_heads as f64 * llm.head_dim as f64 * ctx as f64
        * bytes_per_elem
        / GB
}

pub fn plan(hw: &HardwareConfig, llm: &LlmConfig, weights_gb: f64) -> LlmPlan {
    let budget = (hw.vram_gb - hw.vram_reserve_gb - hw.vram_other_models_gb).max(0.0);
    let mut notes = Vec::new();

    // 1) On privilégie le KV en q8_0 (≈ moitié moins gros, perte de qualité négligeable)
    //    et on ne descend le contexte que si le modèle ne tient vraiment pas.
    let kv_type = "q8_0";
    let bpe = 1.0625; // q8_0 : 34 octets / 32 éléments
    let mut ctx = llm.ctx_tokens;
    let min_ctx = 4096;

    // 2) Tout sur GPU ?
    let fits_full = |ctx: u32| weights_gb + kv_gb(llm, ctx, bpe) + COMPUTE_BUFFER_GB <= budget;
    let mut full = fits_full(ctx);
    while !full && ctx > 8192 {
        ctx -= 2048;
        full = fits_full(ctx);
    }
    if full && ctx < llm.ctx_tokens {
        notes.push(format!("contexte réduit à {ctx} tokens pour tout garder sur le GPU"));
    }

    let (n_gpu_layers, vram_weights, ram_weights);
    if full {
        n_gpu_layers = llm.n_layers + 1; // +1 : couche de sortie
        vram_weights = weights_gb;
        ram_weights = 0.0;
    } else {
        // Offload partiel : le KV reste sur GPU (accès fréquents), on remplit le reste de couches.
        ctx = ctx.max(min_ctx);
        let kv = kv_gb(llm, ctx, bpe);
        let per_layer = weights_gb / llm.n_layers as f64;
        let room = (budget - kv - COMPUTE_BUFFER_GB).max(0.0);
        let layers = ((room / per_layer).floor() as u32).min(llm.n_layers);
        n_gpu_layers = layers;
        vram_weights = layers as f64 * per_layer;
        ram_weights = weights_gb - vram_weights;
        notes.push(format!(
            "offload partiel : {layers}/{} couches sur GPU, {:.1} Go de poids en RAM (génération plus lente)",
            llm.n_layers, ram_weights
        ));
        if ram_weights > hw.ram_gb * 0.8 {
            notes.push("⚠ le modèle dépasse la RAM disponible".into());
        }
    }

    let kv = kv_gb(llm, ctx, bpe);
    LlmPlan {
        n_gpu_layers,
        ctx_tokens: ctx,
        kv_type: kv_type.into(),
        weights_gb,
        kv_gb: kv,
        vram_used_gb: vram_weights + kv + COMPUTE_BUFFER_GB,
        vram_budget_gb: budget,
        ram_used_gb: ram_weights,
        full_offload: full,
        notes,
    }
}

/// Arguments de ligne de commande pour `llama-server`.
pub fn llama_server_args(plan: &LlmPlan, model_path: &str, port: u16, extra: &[String]) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "-m".into(), model_path.into(),
        "--port".into(), port.to_string(),
        "-c".into(), plan.ctx_tokens.to_string(),
        "-ngl".into(), plan.n_gpu_layers.to_string(),
        "--flash-attn".into(), "on".into(),
        "--cache-type-k".into(), plan.kv_type.clone(),
        "--cache-type-v".into(), plan.kv_type.clone(),
        "--parallel".into(), "1".into(),
        "--jinja".into(),
    ];
    a.extend(extra.iter().cloned());
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw() -> HardwareConfig {
        HardwareConfig { vram_gb: 15.0, ram_gb: 64.0, vram_reserve_gb: 1.0, vram_other_models_gb: 1.0 }
    }

    #[test]
    fn model_12b_q5_tient_entierement_sur_gpu() {
        let p = plan(&hw(), &LlmConfig::default(), 8.7);
        assert!(p.full_offload, "{p:?}");
        assert_eq!(p.ctx_tokens, 16384);
        assert!(p.vram_used_gb <= p.vram_budget_gb);
    }

    #[test]
    fn modele_32b_q4_est_offload_partiel() {
        let llm = LlmConfig { n_layers: 64, n_kv_heads: 8, head_dim: 128, ..LlmConfig::default() };
        let p = plan(&hw(), &llm, 19.0);
        assert!(!p.full_offload);
        assert!(p.n_gpu_layers > 0 && p.n_gpu_layers < 64, "{p:?}");
        assert!(p.vram_used_gb <= p.vram_budget_gb + 0.01, "{p:?}");
        assert!(p.ram_used_gb > 0.0);
    }

    #[test]
    fn contexte_reduit_si_juste() {
        // 13 Go de poids : ne tient pas avec 16k de contexte mais tient avec moins.
        let llm = LlmConfig { n_layers: 48, n_kv_heads: 8, head_dim: 128, ..LlmConfig::default() };
        let p = plan(&hw(), &llm, 11.4);
        assert!(p.full_offload, "{p:?}");
        assert!(p.ctx_tokens < 16384);
    }
}
