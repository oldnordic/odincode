//! Numerical Array Operations and Utilities

use anyhow::{anyhow, Result};
use itertools::Itertools;
use ndarray::{s, Array1, Array2, Axis};
use std::f64;

pub struct NDArrayOperations;

impl NDArrayOperations {
    /// Normalize array using min-max scaling
    pub fn min_max_normalize(array: &Array2<f64>) -> Result<Array2<f64>> {
        let min = array.map_axis(Axis(0), |view| {
            view.iter().fold(f64::INFINITY, |acc, &x| acc.min(x))
        });
        let max = array.map_axis(Axis(0), |view| {
            view.iter().fold(f64::NEG_INFINITY, |acc, &x| acc.max(x))
        });

        let range = &max - &min;
        let normalized = array.mapv(|x| x) - &min;
        let normalized = normalized / &range.mapv(|x| if x == 0.0 { 1.0 } else { x });

        Ok(normalized)
    }

    /// Standardize array using z-score normalization
    pub fn z_score_standardize(array: &Array2<f64>) -> Result<Array2<f64>> {
        let mean = array.mean_axis(Axis(0)).unwrap();
        let std = array.std_axis(Axis(0), 0.0);

        let standardized = array.mapv(|x| x) - &mean;
        let standardized = standardized / &std.mapv(|x| if x == 0.0 { 1.0 } else { x });

        Ok(standardized)
    }

    /// Apply logarithmic transformation to array
    pub fn log_transform(array: &Array2<f64>) -> Result<Array2<f64>> {
        let transformed = array.mapv(|x| if x > 0.0 { x.ln() } else { 0.0 });
        Ok(transformed)
    }

    /// Apply power transformation (Box-Cox like)
    pub fn power_transform(array: &Array2<f64>, lambda: f64) -> Result<Array2<f64>> {
        let transformed = if lambda.abs() < 1e-10 {
            array.mapv(|x| if x > 0.0 { x.ln() } else { 0.0 })
        } else {
            array.mapv(|x| {
                if x > 0.0 {
                    (x.powf(lambda) - 1.0) / lambda
                } else {
                    0.0
                }
            })
        };
        Ok(transformed)
    }

    /// Create polynomial features from existing features
    pub fn polynomial_features(array: &Array2<f64>, degree: usize) -> Result<Array2<f64>> {
        let n_samples = array.shape()[0];
        let n_features = array.shape()[1];

        // Calculate number of polynomial features
        let mut n_poly_features = 0;
        for d in 1..=degree {
            n_poly_features += (0..n_features).combinations_with_replacement(d).count();
        }

        let mut poly_features = Array2::zeros((n_samples, n_poly_features));
        let mut col_idx = 0;

        for d in 1..=degree {
            let combos = (0..n_features).combinations_with_replacement(d);
            for combo in combos {
                let mut feature = Array1::ones(n_samples);
                for &idx in &combo {
                    feature = feature * array.column(idx);
                }
                poly_features.column_mut(col_idx).assign(&feature);
                col_idx += 1;
            }
        }

        Ok(poly_features)
    }

    /// Apply one-hot encoding to categorical data
    pub fn one_hot_encode(categorical: &Array1<usize>, n_categories: usize) -> Result<Array2<f64>> {
        let n_samples = categorical.len();
        let mut one_hot = Array2::zeros((n_samples, n_categories));

        for (i, &category) in categorical.iter().enumerate() {
            if category < n_categories {
                one_hot[(i, category)] = 1.0;
            }
        }

        Ok(one_hot)
    }

    /// Compute correlation matrix
    pub fn correlation_matrix(array: &Array2<f64>) -> Result<Array2<f64>> {
        let centered = array.mapv(|x| x) - array.mean_axis(Axis(0)).unwrap();
        let cov = centered.t().dot(&centered) / (array.shape()[0] - 1) as f64;

        let std_devs = cov.diag().mapv(|x| x.sqrt());
        let mut outer_std = Array2::zeros((std_devs.len(), std_devs.len()));
        for i in 0..std_devs.len() {
            for j in 0..std_devs.len() {
                outer_std[(i, j)] = std_devs[i] * std_devs[j];
            }
        }

        let corr = cov / &outer_std.mapv(|x| if x == 0.0 { 1.0 } else { x });
        Ok(corr)
    }

    /// Perform principal component analysis (PCA)
    pub fn pca(
        array: &Array2<f64>,
        n_components: usize,
    ) -> Result<(Array2<f64>, Array2<f64>, Array1<f64>)> {
        let centered = array.mapv(|x| x) - array.mean_axis(Axis(0)).unwrap();
        let _cov = centered.t().dot(&centered) / (array.shape()[0] - 1) as f64;

        // Use eigen decomposition (simplified - in practice use specialized linear algebra libraries)
        let _eigenvalues: Array1<f64> = Array1::zeros(n_components);
        let _eigenvectors: Array2<f64> = Array2::zeros((array.shape()[1], n_components));

        // For demonstration, create simplified PCA components
        // In a real implementation, use proper eigen decomposition
        let components: Array2<f64> = Array2::zeros((array.shape()[1], n_components));
        let explained_variance: Array1<f64> = Array1::ones(n_components);

        let transformed = centered.dot(&components);

        Ok((transformed, components, explained_variance))
    }

    /// Apply feature scaling using robust scaling (median and IQR)
    pub fn robust_scale(array: &Array2<f64>) -> Result<Array2<f64>> {
        let mut medians = Vec::new();
        let mut iqrs = Vec::new();

        for col_idx in 0..array.shape()[1] {
            let col = array.column(col_idx);
            let mut sorted_col = col.to_vec();
            sorted_col.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let median = if sorted_col.is_empty() {
                0.0
            } else if sorted_col.len() % 2 == 0 {
                (sorted_col[sorted_col.len() / 2 - 1] + sorted_col[sorted_col.len() / 2]) / 2.0
            } else {
                sorted_col[sorted_col.len() / 2]
            };

            let q75_idx = (sorted_col.len() as f64 * 0.75) as usize;
            let q25_idx = (sorted_col.len() as f64 * 0.25) as usize;
            let q75 = sorted_col.get(q75_idx).unwrap_or(&0.0);
            let q25 = sorted_col.get(q25_idx).unwrap_or(&0.0);
            let iqr = q75 - q25;

            medians.push(median);
            iqrs.push(if iqr == 0.0 { 1.0 } else { iqr });
        }

        let medians_array = Array1::from(medians);
        let iqrs_array = Array1::from(iqrs);

        let scaled = array.mapv(|x| x) - medians_array;
        let scaled = scaled / iqrs_array;

        Ok(scaled)
    }

    /// Create interaction features between columns
    pub fn interaction_features(array: &Array2<f64>) -> Result<Array2<f64>> {
        let n_samples = array.shape()[0];
        let n_features = array.shape()[1];
        let n_interactions = (n_features * (n_features - 1)) / 2;

        let mut interactions = Array2::zeros((n_samples, n_interactions));
        let mut col_idx = 0;

        for i in 0..n_features {
            for j in (i + 1)..n_features {
                let col_i = array.column(i);
                let col_j = array.column(j);
                let interaction = col_i
                    .iter()
                    .zip(col_j.iter())
                    .map(|(&x, &y)| x * y)
                    .collect::<Vec<f64>>();
                let interaction_array = Array1::from(interaction);
                interactions.column_mut(col_idx).assign(&interaction_array);
                col_idx += 1;
            }
        }

        Ok(interactions)
    }

    /// Apply binning/discretization to continuous features
    pub fn bin_features(array: &Array2<f64>, n_bins: usize) -> Result<Array2<f64>> {
        let mut binned = Array2::zeros(array.dim());

        for col_idx in 0..array.shape()[1] {
            let col = array.column(col_idx);
            let min = col
                .into_iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let max = col
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let bin_width = if n_bins > 0 {
                (max - min) / n_bins as f64
            } else {
                0.0
            };

            for (row_idx, &value) in col.iter().enumerate() {
                let bin_idx = if bin_width > 0.0 {
                    ((value - min) / bin_width) as usize
                } else {
                    0
                };
                let safe_bin_idx = bin_idx.min(n_bins.saturating_sub(1));
                binned[(row_idx, col_idx)] = safe_bin_idx as f64;
            }
        }

        Ok(binned)
    }

    /// Compute pairwise distances between samples
    pub fn pairwise_distances(array: &Array2<f64>, metric: &str) -> Result<Array2<f64>> {
        let n_samples = array.shape()[0];
        let mut distances = Array2::zeros((n_samples, n_samples));

        for i in 0..n_samples {
            for j in 0..n_samples {
                if i == j {
                    distances[(i, j)] = 0.0;
                } else {
                    let sample_i = array.row(i);
                    let sample_j = array.row(j);

                    let distance = match metric {
                        "euclidean" => {
                            let diff = &sample_i - &sample_j;
                            diff.mapv(|x| x * x).sum().sqrt()
                        }
                        "manhattan" => {
                            let diff = &sample_i - &sample_j;
                            diff.mapv(|x| x.abs()).sum()
                        }
                        "cosine" => {
                            let dot_product = (&sample_i * &sample_j).sum();
                            let norm_i = sample_i.mapv(|x| x * x).sum().sqrt();
                            let norm_j = sample_j.mapv(|x| x * x).sum().sqrt();
                            if norm_i > 0.0 && norm_j > 0.0 {
                                1.0 - (dot_product / (norm_i * norm_j))
                            } else {
                                0.0
                            }
                        }
                        _ => return Err(anyhow!("Unsupported distance metric: {}", metric)),
                    };

                    distances[(i, j)] = distance;
                }
            }
        }

        Ok(distances)
    }

    /// Apply feature selection using variance threshold
    pub fn variance_threshold_selection(
        array: &Array2<f64>,
        threshold: f64,
    ) -> Result<Array2<f64>> {
        let variances = array.var_axis(Axis(0), 0.0);
        let selected_cols: Vec<usize> = variances
            .iter()
            .enumerate()
            .filter(|&(_, &var)| var > threshold)
            .map(|(i, _)| i)
            .collect();

        if selected_cols.is_empty() {
            return Ok(Array2::zeros((array.shape()[0], 0)));
        }

        let mut selected_features = Array2::zeros((array.shape()[0], selected_cols.len()));
        for (i, &col_idx) in selected_cols.iter().enumerate() {
            selected_features
                .column_mut(i)
                .assign(&array.column(col_idx));
        }

        Ok(selected_features)
    }

    /// Create sliding window features for time series data
    pub fn sliding_window_features(array: &Array1<f64>, window_size: usize) -> Result<Array2<f64>> {
        let n_samples = array.len();
        if window_size == 0 {
            return Err(anyhow!("Window size must be greater than 0"));
        }
        let n_windows = n_samples.saturating_sub(window_size) + 1;

        let mut windows = Array2::zeros((n_windows, window_size));

        for i in 0..n_windows {
            let window = array.slice(s![i..i + window_size]);
            windows.row_mut(i).assign(&window);
        }

        Ok(windows)
    }

    /// Compute rolling statistics for time series
    pub fn rolling_statistics(array: &Array1<f64>, window_size: usize) -> Result<Array2<f64>> {
        let n_samples = array.len();
        let mut stats = Array2::zeros((n_samples, 3)); // mean, std, min

        if window_size == 0 {
            return Err(anyhow!("Window size must be greater than 0"));
        }

        for i in 0..n_samples {
            let start = i.saturating_sub(window_size - 1);
            let end = i + 1;
            let window = array.slice(s![start..end]);

            stats[(i, 0)] = window.mean().unwrap_or(0.0);
            stats[(i, 1)] = window.std(0.0);
            stats[(i, 2)] = *window
                .into_iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
        }

        Ok(stats)
    }
}
