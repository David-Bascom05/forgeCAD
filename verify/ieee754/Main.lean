/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754

/-!
# `ieee754-eval`: the model, executed

One query per line on stdin, one answer per line on stdout:

    FMT OP ARGS…

`FMT` is `d` (binary64) or `s` (binary32). Operands are encodings in hexadecimal. `OP` is one of
`add sub mul div fma sqrt neg abs` (answer: the result's encoding in hex), `eq lt le` (answer `0`
or `1`), `i2f N` (a decimal integer, rounded; answer in hex) and `f2i X` (truncated, saturated to
i64, NaN to 0, as Rust's `as i64`; answer in decimal).
-/

open IEEE754

def hexVal (s : String) : Option Nat :=
  s.foldl (fun acc c => acc.bind fun a =>
    if '0' ≤ c ∧ c ≤ '9' then some (16 * a + (c.toNat - '0'.toNat))
    else if 'a' ≤ c ∧ c ≤ 'f' then some (16 * a + (c.toNat - 'a'.toNat + 10))
    else if 'A' ≤ c ∧ c ≤ 'F' then some (16 * a + (c.toNat - 'A'.toNat + 10))
    else none) (some 0)

def toHex (n : Nat) : String := String.ofList (Nat.toDigits 16 n)

def evalIn (f : Format) (op : String) (args : List String) : Option String := do
  let fp (s : String) : Option (FP f) := (hexVal s).map FP.ofBits
  let out (x : FP f) : String := toHex x.toBits
  let b (v : Bool) : String := if v then "1" else "0"
  match op, args with
  | "add", [a, c] => return out (FP.add (← fp a) (← fp c))
  | "sub", [a, c] => return out (FP.sub (← fp a) (← fp c))
  | "mul", [a, c] => return out (FP.mul (← fp a) (← fp c))
  | "div", [a, c] => return out (FP.div (← fp a) (← fp c))
  | "fma", [a, c, d] => return out (FP.fma (← fp a) (← fp c) (← fp d))
  | "sqrt", [a] => return out (FP.sqrt (← fp a))
  | "neg", [a] => return out (FP.neg (← fp a))
  | "abs", [a] => return out (FP.abs (← fp a))
  | "eq", [a, c] => return b (FP.eq (← fp a) (← fp c))
  | "lt", [a, c] => return b (FP.lt (← fp a) (← fp c))
  | "le", [a, c] => return b (FP.le (← fp a) (← fp c))
  | "i2f", [n] => return out (FP.ofInt (f := f) (← n.toInt?))
  | "f2i", [a] => return toString (FP.toIntSat (-2 ^ 63) (2 ^ 63 - 1) (← fp a))
  | _, _ => none

def evalLine (line : String) : String :=
  match line.trimAscii.toString.splitOn " " |>.filter (· ≠ "") with
  | "d" :: op :: args => (evalIn binary64 op args).getD "ERR"
  | "s" :: op :: args => (evalIn binary32 op args).getD "ERR"
  | _ => "ERR"

partial def loop (inp : IO.FS.Stream) (out : IO.FS.Stream) : IO Unit := do
  let line ← inp.getLine
  if line.isEmpty then return
  out.putStrLn (evalLine line)
  loop inp out

def main : IO Unit := do
  let out ← IO.getStdout
  loop (← IO.getStdin) out
  out.flush
