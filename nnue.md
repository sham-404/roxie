# Roxie NNUE Architecture (Blaze)

Roxie evaluates chess positions using a custom, highly optimized, efficiently updatable neural network (NNUE). The network is trained to predict game outcomes and translates those probabilities into a standard centipawn evaluation.

The network weights and biases are quantized into integers for rapid inference and stored in a proprietary binary format (`blaze.nnue`).

---

## 🧠 Network Topology

Roxie utilizes a **HalfKP** (King-Piece) architecture. The perspective of the board is split between the White and Black kings, processing features relative to their positions.

The layer structure is `40960 -> 128*2 -> 16 -> 16 -> 1`.

* **Input Layer (40960):** 64 king squares $\times$ 10 piece types $\times$ 64 piece squares.
* **Hidden Layer 1 (HL1):** 128 neurons per color (256 total). This layer is incrementally updated using an accumulator.
* **Hidden Layer 2 (HL2):** 16 neurons.
* **Hidden Layer 3 (HL3):** 16 neurons.
* **Output Layer:** 1 neuron representing the final evaluation.

---

## ⚡ Incremental Accumulator & HalfKP Features

Recalculating the input layer for every node in the search tree is computationally impossible for a high-performance engine. Roxie solves this using an **Incremental Accumulator**.

Instead of evaluating the board from scratch, Roxie stores the state of `HL1` at each ply in the search tree. When a piece moves, the engine only updates the active features that changed:
* Subtract the weights of the piece leaving its `from` square.
* Add the weights of the piece arriving at its `to` square.
* Subtract the weights of any `captured` piece.

The feature index for any piece is calculated as:
`Feature Index = (King Square * 640) + (Piece Type * 64) + Piece Square`

*Note: If the King moves, the entire accumulator for that side must be refreshed from scratch, as the relative position of every piece to the King has changed.*

---

## 🧮 Quantization & Activation

To maximize Nodes Per Second (NPS), all floating-point math is eliminated during search. The network uses **8-bit Quantization** factor:
$$Q = 2^8 = 256$$

Weights are quantized to `i16` and biases to `i32`. During forward propagation, the dot products are shifted to avoid overflow, ensuring the engine can utilize fast SIMD/AVX2 integer instructions.

**Activation Function:**
Between layers, Roxie uses a `hard_tanh` activation function, which effectively acts as a clipped ReLU clamped between $0$ and $Q$.

**Centipawn Scaling:**
The final output layer produces a quantized float $y$ clamped between $[-0.99999, 0.99999]$. This value is mathematically scaled back into a human-readable centipawn score using the inverse hyperbolic tangent:
$$\text{Score} = 600.0 \times \tanh^{-1}(y)$$

---

## 📦 Binary Format (`blaze.nnue`)

The pre-trained network is packed into a raw binary file for instant loading on engine startup. 

**Structure:**
1.  **Magic Header:** 8 bytes `BLAZE_V@` to validate the file format.
2.  **HL1:** Weights (`INPUT * 128` of `i16`) followed by Biases (`128` of `i32`).
3.  **HL2:** Weights (`256 * 16` of `i16`) followed by Biases (`16` of `i32`).
4.  **HL3:** Weights (`16 * 16` of `i16`) followed by Biases (`16` of `i32`).
5.  **Output:** Weights (`16 * 1` of `i16`) followed by Biases (`1` of `i32`).

---

## 🔬 Training & Data Pipeline

The `blaze.nnue` model was trained using custom supervised learning pipelines. For transparency and reproducibility, the entire training dataset generation and model compilation processes are open-source.

* **Data Preprocessing Pipeline:** [https://www.kaggle.com/code/shamsujith/preprocessing]
* **Training Notebook:** [https://www.kaggle.com/code/shamsujith/halfkp-train]
* **NNUE Binary Extracting:** [https://www.kaggle.com/code/shamsujith/nnue-extract]

# NOTE: The nnue is not quantized here, it is quantized after the extraction process
