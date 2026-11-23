import ForgeBoolCore.RefineIncWF
import ForgeBoolCore.Specs
import ForgeBoolCore.Refine
import ForgeBoolCore.RefineUF
import ForgeBoolCore.HeadBridge
import ForgeBoolCore.RefineRule
import ForgeBoolCore.RefineIndex
import ForgeBoolCore.RefineSort
import ForgeBoolCore.RefineExt
import ForgeBoolCore.RefineSplit
import ForgeBoolCore.RefineLedger
import ForgeBoolCore.RefineMemb
import ForgeBoolCore.RefineClassify
import ForgeBoolCore.RefineKeep
import ForgeBoolCore.RefineZeros
import ForgeBoolCore.RefineSortU32
import ForgeBoolCore.RefineAround
import ForgeBoolCore.RefineTouch
import ForgeBoolCore.RefineBetween
import ForgeBoolCore.RefineFan
import ForgeBoolCore.BridgeTouch
import ForgeBoolCore.EndToEnd
import ForgeBoolCore.Integrate
import ForgeBoolCore.RefineSplitOk
import ForgeBoolCore.RefineCollapse
import ForgeBoolCore.RefineUnify
/-! The refinement lemmas use the standard axioms only. -/
#print axioms forge_bool_core.zero_idx_spec
#print axioms forge_bool_core.is_facet_spec
#print axioms forge_bool_core.le_spec
#print axioms forge_bool_core.find_spec
#print axioms forge_bool_core.union_spec
#print axioms forge_bool_core.new_spec
#print axioms forge_bool_core.flood_spec
#print axioms ForgeBoolCore.UF.head_least
#print axioms forge_bool_core.apply_rule_spec
#print axioms forge_bool_core.with_set_spec
#print axioms forge_bool_core.same_cov_spec
#print axioms forge_bool_core.cmp_cov_spec
#print axioms forge_bool_core.index_find_spec
#print axioms forge_bool_core.sort_by_cov_spec
#print axioms forge_bool_core.h_sort_hashed_spec
#print axioms forge_bool_core.hash_cov_spec
#print axioms forge_bool_core.same_spec
#print axioms forge_bool_core.hash_table_spec
#print axioms forge_bool_core.find_h_spec
#print axioms forge_bool_core.index_new_spec
#print axioms forge_bool_core.cov_spec
#print axioms forge_bool_core.cells_find_spec
#print axioms forge_bool_core.intern_spec
#print axioms forge_bool_core.split_loop0_spec
#print axioms ForgeBoolCore.Dedup.dedup_map
#print axioms forge_bool_core.split_loop1_spec
#print axioms forge_bool_core.split_loop2_spec
#print axioms forge_bool_core.split_loop3_spec
#print axioms forge_bool_core.split_loop4_spec
#print axioms forge_bool_core.split_spec
#print axioms forge_bool_core.facets_toL
#print axioms forge_bool_core.topes_toL
#print axioms forge_bool_core.piece_toL
#print axioms forge_bool_core.splitWF_of
#print axioms forge_bool_core.floodEdges_eq
#print axioms forge_bool_core.memb_loop0_spec
#print axioms forge_bool_core.memb_row_spec
#print axioms forge_bool_core.memb_loop1_spec
#print axioms forge_bool_core.memb_eq
#print axioms forge_bool_core.membership_spec
#print axioms forge_bool_core.classify_outer_spec
#print axioms forge_bool_core.ckAll_eq
#print axioms forge_bool_core.keep_loop_spec
#print axioms forge_bool_core.keep_res_spec
#print axioms forge_bool_core.zeros_spec
#print axioms forge_bool_core.sort_u32_spec
#print axioms forge_bool_core.enc_complete
#print axioms forge_bool_core.enc_unique
#print axioms forge_bool_core.enum_buf_spec
#print axioms forge_bool_core.enum_m_spec
#print axioms forge_bool_core.enum_keep_spec
#print axioms forge_bool_core.enc_iff
#print axioms forge_bool_core.mem_hitsAll
#print axioms forge_bool_core.hitsAll_nodup
#print axioms forge_bool_core.around_enum_spec
#print axioms forge_bool_core.around_loop_spec
#print axioms forge_bool_core.around_spec
#print axioms forge_bool_core.is_edge_spec
#print axioms forge_bool_core.edge_scan_spec
#print axioms forge_bool_core.between_iff
#print axioms forge_bool_core.between_spec
#print axioms forge_bool_core.edges_between_spec
#print axioms forge_bool_core.fan_classes
#print axioms forge_bool_core.bad_vert_spec
#print axioms forge_bool_core.vert_scan_spec
-- group R: the bridges from positions and ids to covectors
#print axioms forge_bool_core.R_result_eq
#print axioms forge_bool_core.R_orient_eq
#print axioms forge_bool_core.R_resOK_of
#print axioms forge_bool_core.result_bridge_proof
#print axioms forge_bool_core.around_bridge_proof
#print axioms forge_bool_core.bad_edge_bridge_proof
#print axioms forge_bool_core.bad_vert_bridge_proof
#print axioms forge_bool_core.new_ext_spec_proof
#print axioms forge_bool_core.core_k_spec_of
#print axioms forge_bool_core.classify_on_spec_of
#print axioms forge_bool_core.core_spec_of
#print axioms forge_bool_core.end_to_end
#print axioms forge_bool_core.encode_spec_proof
#print axioms forge_bool_core.end_to_end_encode
#print axioms forge_bool_core.end_to_end_SKU
-- group S: split_ok
#print axioms forge_bool_core.s_le3_spec
#print axioms forge_bool_core.s_sort_triples_spec
#print axioms forge_bool_core.s_adj_iff
#print axioms forge_bool_core.s_loop0_spec
#print axioms forge_bool_core.s_loop1_spec
#print axioms forge_bool_core.split_ok_spec_proof
#print axioms forge_bool_core.end_to_end_KU
#print axioms forge_bool_core.collapse_spec_proof
#print axioms forge_bool_core.K.collapse_eq
#print axioms forge_bool_core.end_to_end_U
#print axioms forge_bool_core.has_spec
#print axioms forge_bool_core.same_set_spec
#print axioms forge_bool_core.faces_at_spec
#print axioms forge_bool_core.u_join_spec
#print axioms forge_bool_core.u_faces_loop_spec
#print axioms forge_bool_core.u_erel_spec
#print axioms forge_bool_core.u_edges_loop_spec
#print axioms forge_bool_core.unify_spec'
#print axioms forge_bool_core.unify_spec_proof
#print axioms forge_bool_core.end_to_end_proof
#print axioms forge_bool_core.end_to_end_encode_proof
#print axioms forge_bool_core.inc.endToEndF_of
#print axioms forge_bool_core.inc.rows_spec
#print axioms forge_bool_core.inc.row_spec
#print axioms forge_bool_core.inc.body_of_spec
#print axioms forge_bool_core.inc.cls_of_spec
#print axioms forge_bool_core.inc.splitOk_iff
#print axioms forge_bool_core.inc.split_ok_spec
#print axioms forge_bool_core.inc.ins_spec
#print axioms forge_bool_core.inc.split_loop1_spec
#print axioms forge_bool_core.inc.split_loop0_spec
#print axioms forge_bool_core.inc.split_spec
#print axioms forge_bool_core.inc.flood_spec
#print axioms ForgeBoolCore.UF.head_leastI
#print axioms forge_bool_core.inc.membership_loop0_spec
#print axioms forge_bool_core.inc.memb_bridge
#print axioms forge_bool_core.inc.membership_spec
#print axioms forge_bool_core.inc.core_loop0_spec
#print axioms forge_bool_core.inc.cAll_eq
#print axioms forge_bool_core.inc.core_loop1_spec
#print axioms forge_bool_core.inc.keep_eq
#print axioms forge_bool_core.inc.core_k_loop0_spec
#print axioms forge_bool_core.inc.result_eq
#print axioms forge_bool_core.inc.orient_eq
#print axioms forge_bool_core.inc.around_e_spec
#print axioms forge_bool_core.inc.aroundE_len
#print axioms forge_bool_core.inc.core_k_loop1_spec
#print axioms forge_bool_core.inc.edges_find_eq
#print axioms ForgeBoolCore.UF.fan_bad
#print axioms forge_bool_core.inc.around_v_spec
#print axioms forge_bool_core.inc.fans_spec
#print axioms forge_bool_core.inc.bad_vert_spec
#print axioms forge_bool_core.inc.core_k_loop2_spec
#print axioms forge_bool_core.inc.coreMI_nil
#print axioms forge_bool_core.inc.core_k_spec
#print axioms forge_bool_core.inc.core_spec
#print axioms forge_bool_core.inc.incCoreSpec_of
#print axioms forge_bool_core.inc.KI.collapse_pos
#print axioms forge_bool_core.inc.collapse_specI
#print axioms forge_bool_core.inc.incCoreSpec_of_unify
#print axioms forge_bool_core.inc.csr_ok_spec
#print axioms forge_bool_core.inc.count_in_spec
#print axioms forge_bool_core.inc.count_sec_spec
