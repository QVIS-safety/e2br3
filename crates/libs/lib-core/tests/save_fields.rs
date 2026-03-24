#[path = "save_fields/c.rs"]
mod c;
#[path = "save_fields/common.rs"]
mod common;
#[path = "save_fields/d.rs"]
mod d;
#[path = "save_fields/e.rs"]
mod e;
#[path = "save_fields/f.rs"]
mod f;
#[path = "save_fields/g.rs"]
mod g;
#[path = "save_fields/h.rs"]
mod h;
#[path = "save_fields/n.rs"]
mod n;
#[path = "common/mod.rs"]
mod test_common;
/// `tests/save_fields.rs` covers direct create/update persistence correctness.
///
/// Business-field assertion rule:
/// - assert all non-metadata business fields
/// - exclude only `id`, `created_at`, `updated_at`, `created_by`, `updated_by`
///
/// Coverage manifest:
/// - `C.1` -> `c::save_c_1_create`, `c::save_c_1_update`
/// - `C.2` -> `c::save_c_2_create`, `c::save_c_2_update`
/// - `C.2.r` -> `c::save_c_2_r_create`, `c::save_c_2_r_update`
/// - `C.3.1.r` -> `c::save_c_3_1_r_create`, `c::save_c_3_1_r_update`
/// - `C.3.2.r` -> `c::save_c_3_2_r_create`, `c::save_c_3_2_r_update`
/// - `C.4.r` -> `c::save_c_4_r_create`, `c::save_c_4_r_update`
/// - `C.4` -> `c::save_c_4_create`, `c::save_c_4_update`
/// - `C.5` -> `c::save_c_5_create`, `c::save_c_5_update`
/// - `C.5.r` -> `c::save_c_5_r_create`, `c::save_c_5_r_update`
/// - `D.1.2` -> `d::save_d_1_2_create`, `d::save_d_1_2_update`
/// - `D.2.1.r` -> `d::save_d_2_1_r_create`, `d::save_d_2_1_r_update`
/// - `D.7` -> `d::save_d_7_create`, `d::save_d_7_update`
/// - `D.8.r` -> `d::save_d_8_r_create`, `d::save_d_8_r_update`
/// - `D.9` -> `d::save_d_9_create`, `d::save_d_9_update`
/// - `D.9.1.r` -> `d::save_d_9_1_r_create`, `d::save_d_9_1_r_update`
/// - `D.9.2.r` -> `d::save_d_9_2_r_create`, `d::save_d_9_2_r_update`
/// - `D.10` -> `d::save_d_10_create`, `d::save_d_10_update`
/// - `D.10.6.r` -> `d::save_d_10_6_r_create`, `d::save_d_10_6_r_update`
/// - `D.10.7.r` -> `d::save_d_10_7_r_create`, `d::save_d_10_7_r_update`
/// - `E.i` -> `e::save_e_i_create`, `e::save_e_i_update`
/// - `F.r` -> `f::save_f_r_create`, `f::save_f_r_update`
/// - `G.k` -> `g::save_g_k_create`, `g::save_g_k_update`
/// - `G.k.2.3.r` -> `g::save_g_k_2_3_r_create`, `g::save_g_k_2_3_r_update`
/// - `G.k.4.r` -> `g::save_g_k_4_r_create`, `g::save_g_k_4_r_update`
/// - `G.k.6.r` -> `g::save_g_k_6_r_create`, `g::save_g_k_6_r_update`
/// - `G.k.8.r` -> `g::save_g_k_8_r_create`, `g::save_g_k_8_r_update`
/// - `G.k.10` -> `g::save_g_k_10_create`, `g::save_g_k_10_update`
/// - `G.k.9.i` -> `g::save_g_k_9_i_create`, `g::save_g_k_9_i_update`
/// - `G.k.9.i.2.r` -> `g::save_g_k_9_i_2_r_create`, `g::save_g_k_9_i_2_r_update`
/// - `H.1.2.4` -> `h::save_h_1_2_4_create`, `h::save_h_1_2_4_update`
/// - `H.3.r` -> `h::save_h_3_r_create`, `h::save_h_3_r_update`
/// - `H.5.r` -> `h::save_h_5_r_create`, `h::save_h_5_r_update`
/// - `N` -> `n::save_n_create`, `n::save_n_update`
const _: () = ();
