package SyntheticInterchange {
package KernspanStage1DraftOntology {
    enum def Comparator { enum GreaterOrEqual; enum LessOrEqual; enum Equal; }
    enum def LegacyQuantity { enum Dimensionless; enum Length; }
    enum def LegacyUnit { enum One; enum Percent; enum Metre; }
    enum def AffiliationKind { enum Unknown; enum Internal; enum External; }
    enum def ContextKind { enum Description; enum Assumption; enum AcceptedConstraint; }
    attribute def RatioValue :> MeasurementReferences::DimensionOneValue;
    attribute percent : MeasurementReferences::DimensionOneUnit {
        attribute :>> unitConversion : MeasurementReferences::ConversionByConvention {
            attribute :>> referenceUnit = MeasurementReferences::one;
            attribute :>> conversionFactor = 0.01;
        }
    }
    part def RecordDraft {
        attribute id : ScalarValues::Integer;
        attribute displayName : ScalarValues::String;
        ref part scope : SystemDraft;
    }
    part def SystemDraft :> RecordDraft { attribute purpose : ScalarValues::String[0..1]; }
    part def MissionDraft :> RecordDraft { attribute shortLabel : ScalarValues::String[0..1]; }
    part def CapabilityDraft :> RecordDraft { ref part contributesTo : MissionDraft[0..*]; }
    part def ObjectiveDraft :> RecordDraft {
        attribute outcome : ScalarValues::String[0..1];
        ref part 'subject' : RecordDraft[0..1];
    }
    part def MeasureDraft :> RecordDraft {
        ref part forObjective : ObjectiveDraft;
        attribute quantitySelection : LegacyQuantity[0..1];
        attribute unitSelection : LegacyUnit[0..1];
        attribute criterion : Quantities::ScalarQuantityValue[0..1];
        attribute comparator : Comparator[0..1];
        attribute assessmentMethod : ScalarValues::String[0..1];
        attribute conditions : ScalarValues::String[0..1];
        attribute uncertainty : ScalarValues::String[0..1];
        ref calc calculation : Calculations::Calculation[0..1];
    }
    part def StakeholderDraft :> RecordDraft { attribute interest : ScalarValues::String[0..1]; }
    part def ExpectationDraft :> RecordDraft {
        ref part expressedBy : StakeholderDraft[0..1];
        ref part concernsSystem : SystemDraft[0..1];
        attribute text : ScalarValues::String[0..1];
    }
    part def EnvironmentDraft :> RecordDraft;
    part def OrganizationDraft :> RecordDraft;
    part def ActorDraft :> RecordDraft;
    part def ContextDraft :> RecordDraft {
        attribute kind : ContextKind;
        attribute text : ScalarValues::String;
    }
    part def AffiliationDraft {
        ref part organization : OrganizationDraft;
        attribute relationship : AffiliationKind;
    }
}
    part <ks_r_1> system : KernspanStage1DraftOntology::SystemDraft {
        attribute :>> id = 1;
        attribute :>> displayName = "Original synthetic QA";
        ref part :>> scope = system;
        attribute :>> purpose = "A small source-preserving interchange example.";
    }
    action inventedRunner;
}
