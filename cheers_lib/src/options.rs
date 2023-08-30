#[derive(Clone, Copy)]
pub struct SearchOptions {
    pub threads: usize,
    pub tt_size_mb: usize,
    pub nmp_depth: i8,
    pub nmp_const_reduction: i8,
    pub nmp_linear_divisor: i8,
    pub see_pruning_depth: i8,
    pub see_capture_margin: i16,
    pub see_quiet_margin: i16,
    pub pvs_fulldepth: i8,
    pub delta_pruning_margin: i16,
    pub fp_margin_1: i16,
    pub fp_margin_2: i16,
    pub fp_margin_3: i16,
    pub rfp_margin: i16,
    pub lmp_depth: i8,
    pub iir_depth: i8,
}

pub const NMP_DEPTH: i8 = 1;
pub const NMP_CONST_REDUCTION: i8 = 3;
pub const NMP_LINEAR_DIVISOR: i8 = 3;
pub const SEE_PRUNING_DEPTH: i8 = 9;
pub const SEE_CAPTURE_MARGIN: i16 = -47;
pub const SEE_QUIET_MARGIN: i16 = -61;
pub const PVS_FULLDEPTH: i8 = 1;
pub const DELTA_PRUNING_MARGIN: i16 = 183;
pub const FP_MARGIN_1: i16 = 114;
pub const FP_MARGIN_2: i16 = 250;
pub const FP_MARGIN_3: i16 = 509;
pub const RFP_MARGIN: i16 = 198;
pub const LMP_DEPTH: i8 = 9;
pub const IIR_DEPTH: i8 = 7;

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            threads: 1,
            tt_size_mb: 8,
            nmp_depth: NMP_DEPTH,
            nmp_const_reduction: NMP_CONST_REDUCTION,
            nmp_linear_divisor: NMP_LINEAR_DIVISOR,
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
            iir_depth: IIR_DEPTH,
        }
    }
}
