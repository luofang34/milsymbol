//! The typed metadata view of a rendered symbol.

use super::Metadata;
use crate::metadata::Field;

impl Metadata {
    pub(crate) fn from_internal(md: &crate::metadata::Metadata) -> Self {
        let flag = |f: Option<bool>| f == Some(true);
        Metadata {
            affiliation: md.affiliation.known(),
            base_affiliation: md.base_affiliation.known(),
            dimension: md.dimension.known(),
            base_dimension: md.base_dimension.known(),
            dimension_unknown: md.dimension_unknown,
            context: md.context,
            condition: md.condition,
            not_present: !md.notpresent.is_empty(),
            echelon: md.echelon.known(),
            mobility: md.mobility.known(),
            amplifier_unknown: md.mobility == Field::Missing,
            leadership: md.flags.leadership,
            edition: md.flags.edition,
            geometry: md.geometry(),
            headquarters: md.headquarters,
            task_force: md.task_force,
            feint_dummy: flag(md.flags.feint_dummy),
            installation: md.installation,
            activity: md.activity,
            space: md.space,
            unit: md.unit,
            land_equipment: flag(md.flags.landequipment),
            dismounted: flag(md.flags.dismounted),
            cyberspace: flag(md.flags.cyberspace),
            control_measure: flag(md.flags.control_measure),
            civilian: md.civilian,
            suspect: flag(md.flags.suspect),
            joker: md.joker,
            faker: md.faker,
            frame: md.frame,
            fill: md.fill,
            std2525: md.std2525,
            numeric_sidc: md.number_sidc,
        }
    }
}
