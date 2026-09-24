# Roxie NNUE Architecture (Blaze)

Roxie evaluates chess positions using a custom, highly optimized, efficiently updatable neural network (NNUE). The network is trained to predict game outcomes and translates those probabilities into a standard centipawn evaluation.

The network weights and biases are quantized into integers for rapid inference and stored in a proprietary binary format (`blaze.nnue`).

---

## 🧠 Network Topology

Roxie utilizes a **Bucketed 768** architecture. The perspective of the board is split between the White and Black kings, processing features relative to their positions. To optimize learning and capacity, king placements are divided into 8 distinct buckets (`KING_BUCKETS`) with horizontal board mirroring (`MIRROR_MASK`) applied.

The layer structure is `6144 -> 256*2 -> 16 -> 32 -> 1`.

* **Input Layer (6144):** 8 king buckets * 12 piece types * 64 piece squares.
* **Hidden Layer 1 (HL1):** 256 neurons per color (512 total). This layer is incrementally updated using an accumulator.
* **Hidden Layer 2 (HL2):** 16 neurons.
* **Hidden Layer 3 (HL3):** 32 neurons.
* **Output Layer:** 1 neuron representing the final evaluation.

---

## ⚡ Incremental Accumulator & Bucketed Features

Recalculating the input layer for every node in the search tree is computationally impossible for a high-performance engine. Roxie solves this using an **Incremental Accumulator**.

Instead of evaluating the board from scratch, Roxie stores the state of `HL1` at each ply in the search tree. When a piece moves, the engine only updates the active features that changed:
* Subtract the weights of the piece leaving its `from` square.
* Add the weights of the piece arriving at its `to` square.
* Subtract the weights of any `captured` piece.

The feature index for any piece is calculated based on the king's bucket, the piece index, and the mirrored square:
`Feature Index = (King Bucket * 768) + (Piece Index * 64) + Mirrored Square`

*Note: The entire accumulator for a side is only refreshed from scratch if the King moves to a different bucket or its horizontal mirror mask changes. Otherwise, king moves within the same bucket are treated as normal fast incremental updates.*

---

## 🧮 Quantization & Activation

To maximize Nodes Per Second (NPS), all floating-point math is eliminated during search. The network uses **8-bit Quantization** where factor Q = 256.

All weights and biases across all layers are quantized to `i16`. During forward propagation, the dot products are shifted to avoid overflow, ensuring the engine can utilize fast SIMD (AVX2 and NEON) integer instructions.

**Activation Function:**
Between layers, Roxie uses a SIMD-optimized **SCReLU (Squared Clipped ReLU)** activation function. The intermediate values are clamped between 0 and Q (256), squared, and then bit-shifted appropriately.

**Centipawn Scaling:**
The final output layer produces an integer which is converted to a float and divided by Q. This value is mathematically scaled back into a human-readable centipawn score using a linear multiplier of 400.0:
$$Score = \left( \frac{Output}{Q} \right) \times 400.0$$

---

## 📦 Binary Format (`blaze.nnue`)

The pre-trained network is packed into a raw binary file for instant loading on engine startup. 

**Structure:**
1.  **Magic Header:** 8 bytes `BLAZE_V#` to validate the file format.
2.  **HL1:** Weights (`6144 * 256` of `i16`) followed by Biases (`256` of `i16`).
3.  **HL2:** Weights (`512 * 16` of `i16`) followed by Biases (`16` of `i16`).
4.  **HL3:** Weights (`16 * 32` of `i16`) followed by Biases (`32` of `i16`).
5.  **Output:** Weights (`32 * 1` of `i16`) followed by Biases (`1` of `i16`).

---
