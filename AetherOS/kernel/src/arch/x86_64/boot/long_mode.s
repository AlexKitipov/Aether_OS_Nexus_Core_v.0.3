/*
 * Placeholder long mode assembly hook.
 * Long mode control register/MSR toggles are performed in Rust in
 * `long_mode_init`, while this symbol keeps an explicit assembly path
 * available for early-boot extensions.
 */

.section .text
.global __aether_long_mode_stub
__aether_long_mode_stub:
    ret
