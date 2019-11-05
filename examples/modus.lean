/-! Documentation header for lumpy-leandoc modus example.
    This file requires fixes present in the lean community edition.
 -/
variables {P Q : Prop}
/-- Modus Tollens
```latex
\begin{centering}
    \begin{prooftree}
      \AxiomC{P → Q}
      \AxiomC{¬Q}
      \BinaryInfC{¬P}
    \end{prooftree}
\end{centering}
```

Example usage:
```lean
def impl_double_neg_intro {P Q : Prop} (_ : P → Q) : ¬¬P → ¬¬Q :=
λ (_ : ¬¬P) (_ : ¬Q),
  have ¬Q → ¬P, from (modus_tollens ‹P → Q›),
  have ¬P, from ‹¬Q → ¬P› ‹¬Q›,
  show false, from ‹¬¬P› ‹¬P›
```
-/
def modus_tollens (_: P → Q) (_: ¬Q) : ¬P
:= assume _ : P,
   show false, from ‹Q → false› (‹P → Q› ‹P›)


/-- Modus Ponens
```latex
    \centering
    \begin{prooftree}
      \AxiomC{P → Q}
      \AxiomC{P}
      \BinaryInfC{Q}
    \end{prooftree}
```-/
def modus_ponens (_ : P → Q) (_ : P) : Q
:= ‹P → Q› ‹P›



/-- Given a function from `P → Q` return a function from `¬¬P → ¬¬Q` -/
def impl_double_neg_intro {P Q : Prop} (_ : P → Q) : ¬¬P → ¬¬Q :=
  λ (_ : ¬¬P) (_ : ¬Q),
  have ¬Q → ¬P, from (modus_tollens ‹P → Q›),
  have ¬P, from ‹¬Q → ¬P› ‹¬Q›,
  ‹¬¬P› ‹¬P›
