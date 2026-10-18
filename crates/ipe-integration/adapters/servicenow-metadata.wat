;; metadata-field-v1: classify(pointer, length, kind, required) -> semantic role.
;; Host owns transport, secrets, identity and validation. This module has no imports.
(module
  (memory (export "memory") 1 1)
  (data (i32.const 1024) "stateassigned_toassignment_groupactivesys_updated_on")
  (func $equals (param $p i32) (param $len i32) (param $s i32) (param $n i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $len) (local.get $n)) (then (return (i32.const 0))))
    (block $done (loop $next
      (br_if $done (i32.eq (local.get $i) (local.get $len)))
      (if (i32.ne (i32.load8_u (i32.add (local.get $p) (local.get $i)))
                  (i32.load8_u (i32.add (local.get $s) (local.get $i))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $next)))
    (i32.const 1))
  (func (export "classify") (param $p i32) (param $len i32) (param $kind i32) (param $required i32) (result i32)
    (if (call $equals (local.get $p) (local.get $len) (i32.const 1024) (i32.const 5)) (then (return (i32.const 1))))
    (if (call $equals (local.get $p) (local.get $len) (i32.const 1029) (i32.const 11)) (then (return (i32.const 2))))
    (if (call $equals (local.get $p) (local.get $len) (i32.const 1040) (i32.const 16)) (then (return (i32.const 3))))
    (if (call $equals (local.get $p) (local.get $len) (i32.const 1056) (i32.const 6)) (then (return (i32.const 4))))
    (if (call $equals (local.get $p) (local.get $len) (i32.const 1062) (i32.const 14)) (then (return (i32.const 5))))
    (i32.const 0)))
