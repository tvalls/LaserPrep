# ADR 0002: Content classification uses classical heuristics only — no AI/ML

## Status

Accepted (non-negotiable product constraint)

## Context

LaserPrep must decide, per image, which vectorization strategy to apply
(portrait, animal, logo, drawing, landscape, etc.). A machine-learning
classifier (a small CNN, a pretrained model, an ONNX model, a cloud
vision API) would be the conventional way to solve this class of problem.

## Decision

LaserPrep will **never** use machine learning or AI models for any part of
its pipeline, including content classification. This is a hard product
constraint, not a technical trade-off open to reconsideration by future
contributors.

Classification is implemented entirely with deterministic, classical
image-processing heuristics:

- luminance/color histograms (contrast, dynamic range, bimodality)
- edge density (Sobel/Canny via `imageproc`)
- color clustering via k-means (`kmeans_colors`) — a deterministic
  algorithm, not a trained model
- saturation/hue distribution
- uniform-background ratio (largest low-variance connected region at the
  image borders)
- mirror symmetry score
- aspect ratio and largest central low-saturation blob (position/geometry
  heuristic — no learned object/face detection)

Each category has a hand-weighted scoring function over these features.
Confidence is a normalization of the winning score against the runner-up,
never a model's softmax/probability output.

## Consequences

- No ONNX Runtime, no pretrained weights, no Model Manager, no local or
  remote inference dependency will ever be added to this project.
- If a future technical problem seems to require ML to solve well, the
  correct response is to document the limitation here (or in a new ADR)
  and propose a heuristic alternative — not to introduce ML.
- Classification weights are calibrated by hand and validated against the
  regression fixture suite (`tests/fixtures/`), not by training.
