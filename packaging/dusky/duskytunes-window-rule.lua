-- Duskytunes — Dusky-native main-window policy.
--
-- Dusky Settings, Duskywalls and Duskyvm are ordinary toplevel windows; Dusky floats and sizes them
-- with an `hl.window_rule` before map. Duskytunes follows that same compositor-owned pattern.
-- Match the exact main title so the separate "Duskytunes Mini" surface is never caught by this rule.
hl.window_rule({
    name   = "float-duskytunes-custom",
    match  = { title = "^(Duskytunes)$" },
    float  = true,
    size   = { 1760, 1000 },
    center = true,
})
