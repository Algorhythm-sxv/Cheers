

#[derive(Clone, Copy)]
pub struct SearchOptions {
    pub tt_size_mb: usize,
    pub nmp_depth: i32,
    pub nmp_reduction: i32,
    pub see_pruning_depth: i32,
    pub see_capture_margin: i32,
    pub see_quiet_margin: i32,
    pub pvs_fulldepth: i32,
    pub delta_pruning_margin: i32,
    pub fp_margin_1: i32,
    pub fp_margin_2: i32,
    pub fp_margin_3: i32,
    pub rfp_margin: i32,
    pub lmp_depth: i32,
    pub lmp_margin: i32,
    pub iir_depth: i32,
}

pub const NMP_DEPTH: i32 = 2;
pub const NMP_REDUCTION: i32 = 5;
pub const SEE_PRUNING_DEPTH: i32 = 6;
pub const SEE_CAPTURE_MARGIN: i32 = 59;
pub const SEE_QUIET_MARGIN: i32 = 39;
pub const PVS_FULLDEPTH: i32 = 1;
pub const DELTA_PRUNING_MARGIN: i32 = 91;
pub const FP_MARGIN_1: i32 = 115;
pub const FP_MARGIN_2: i32 = 344;
pub const FP_MARGIN_3: i32 = 723;
pub const RFP_MARGIN: i32 = 106;
pub const LMP_DEPTH: i32 = 1;
pub const LMP_MARGIN: i32 = 2;
pub const IIR_DEPTH: i32 = 4;

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            tt_size_mb: 8,
            nmp_depth: NMP_DEPTH,
            nmp_reduction: NMP_REDUCTION,
            see_pruning_depth: SEE_PRUNING_DEPTH,
            see_capture_margin: SEE_CAPTURE_MARGIN,
            see_quiet_margin: SEE_QUIET_MARGIN,
            pvs_fulldepth: PVS_FULLDEPTH,
            delta_pruning_margin: DELTA_PRUNING_MARGIN,
            fp_margin_1: FP_MARGIN_1,
            fp_margin_2: FP_MARGIN_2,
            fp_margin_3: FP_MARGIN_3,
            rfp_margin: RFP_MARGIN,
            lmp_depth: LMP_DEPTH,
            lmp_margin: LMP_MARGIN,
            iir_depth: IIR_DEPTH,
        }
    }
}