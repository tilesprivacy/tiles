//! the models onboarding offers, and which one fits this machine

use serde::Serialize;

/// One model Tiles offers to download.
#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    pub id: &'static str,
    pub label: &'static str,
    pub repo: &'static str,
    pub quant: &'static str,
}

impl Candidate {
    /// the `repo:quant` spec used as the model's identity everywhere else
    pub fn spec(&self) -> String {
        format!("{}:{}", self.repo, self.quant)
    }
}

/// Gemma 4, smallest first
pub const LINEUP: [Candidate; 5] = [
    Candidate {
        id: "gemma-4-e2b",
        label: "Gemma 4 E2B",
        repo: "unsloth/gemma-4-E2B-it-GGUF",
        quant: "Q4_K_M",
    },
    Candidate {
        id: "gemma-4-e4b",
        label: "Gemma 4 E4B",
        repo: "unsloth/gemma-4-E4B-it-GGUF",
        quant: "Q4_K_M",
    },
    Candidate {
        id: "gemma-4-12b",
        label: "Gemma 4 12B",
        repo: "unsloth/gemma-4-12b-it-GGUF",
        quant: "Q4_K_M",
    },
    Candidate {
        id: "gemma-4-26b-a4b",
        label: "Gemma 4 26B-A4B",
        repo: "unsloth/gemma-4-26B-A4B-it-GGUF",
        quant: "UD-Q4_K_M",
    },
    Candidate {
        id: "gemma-4-31b",
        label: "Gemma 4 31B",
        repo: "unsloth/gemma-4-31B-it-GGUF",
        quant: "Q4_K_M",
    },
];

/// Headroom kept free on the device for whatever else is using it.
const BUDGET_SHARE: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// all of it on the gpu
    Fits,
    /// a mixture-of-experts model whose experts wait on the cpu, which only
    /// costs a little speed since few are active per token
    ExpertsOnCpu,
    /// part of the model on the cpu, and slow for it
    TooBig,
    /// no estimate, usually offline
    Unknown,
}

/// What a model needs, from the inference server's header estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    pub vram_bytes: u64,
    /// the share that can stay on the cpu, non-zero only for moe models
    pub expert_bytes: u64,
}

pub fn fit(need: Option<Need>, free_bytes: u64) -> Fit {
    let Some(need) = need else {
        return Fit::Unknown;
    };
    let budget = (free_bytes as f64 * BUDGET_SHARE) as u64;
    if need.vram_bytes <= budget {
        Fit::Fits
    } else if need.expert_bytes > 0 && need.vram_bytes - need.expert_bytes <= budget {
        Fit::ExpertsOnCpu
    } else {
        Fit::TooBig
    }
}

/// The largest model that fits whole; failing that one whose experts can wait
/// on the cpu; failing that the smallest. `fits` is in lineup order.
pub fn recommend(fits: &[Fit]) -> usize {
    let largest = |wanted: Fit| fits.iter().rposition(|fit| *fit == wanted);
    largest(Fit::Fits)
        .or_else(|| largest(Fit::ExpertsOnCpu))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn need(vram: f64, experts: f64) -> Option<Need> {
        Some(Need {
            vram_bytes: (vram * GIB as f64) as u64,
            expert_bytes: (experts * GIB as f64) as u64,
        })
    }

    /// the numbers measured and estimated for the lineup at 32k context
    fn lineup() -> [Option<Need>; 5] {
        [
            need(1.9, 0.0),
            need(3.7, 0.0),
            need(8.8, 0.0),
            need(17.6, 13.4),
            need(23.4, 0.0),
        ]
    }

    fn fits_on(free_gib: f64) -> Vec<Fit> {
        let free = (free_gib * GIB as f64) as u64;
        lineup().iter().map(|need| fit(*need, free)).collect()
    }

    #[test]
    fn a_16gb_card_gets_the_12b_and_runs_the_moe_with_experts_on_cpu() {
        let fits = fits_on(14.2);
        assert_eq!(
            fits,
            [
                Fit::Fits,
                Fit::Fits,
                Fit::Fits,
                Fit::ExpertsOnCpu,
                Fit::TooBig
            ]
        );
        assert_eq!(LINEUP[recommend(&fits)].id, "gemma-4-12b");
    }

    #[test]
    fn a_24gb_card_gets_the_largest_that_fits_whole() {
        let fits = fits_on(23.5);
        assert_eq!(LINEUP[recommend(&fits)].id, "gemma-4-26b-a4b");
    }

    #[test]
    fn a_small_card_still_gets_something_that_runs() {
        assert_eq!(LINEUP[recommend(&fits_on(3.0))].id, "gemma-4-e2b");
        // nothing fits and there is no moe to fall back on
        assert_eq!(LINEUP[recommend(&fits_on(0.5))].id, "gemma-4-e2b");
    }

    #[test]
    fn no_estimate_is_unknown_not_a_guess() {
        assert_eq!(fit(None, 16 * GIB), Fit::Unknown);
        assert_eq!(recommend(&[Fit::Unknown; 5]), 0);
    }
}
