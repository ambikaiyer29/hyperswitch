use error_stack::ResultExt;
use common_enums::enums;
use serde::{Deserialize, Serialize};
use common_utils::ext_traits::{Encode, ValueExt};
use masking::Secret;
use common_utils::types::{StringMinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};

use hyperswitch_interfaces::errors;
use router_env::logger;
use crate::{types::{RefundsResponseRouterData, ResponseRouterData}, utils, utils::PaymentsAuthorizeRequestData};
use crate::utils::{CardData as cd, RouterData as rd};

//TODO: Fill the struct with respective fields
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ChaseorbitalRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    From<(
        StringMinorUnit,
        T,
    )> for ChaseorbitalRouterData<T>
{
    fn from(
        (amount, item): (
            StringMinorUnit,
            T,
        ),
    ) -> Self {
         //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChaseorbitalPaymentsRequest {
    amount: Amount,
    source: Source,
    transaction_details: TransactionDetails,
    merchant_details:  MerchantDetails,
    transaction_interaction: TransactionInteraction,
}

#[derive(Debug, Serialize)]
#[serde(tag = "sourceType")]
pub enum Source {
    PaymentCard {
        card: CardData,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    card_data: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayToken {
    signature: Secret<String>,
    signed_message: Secret<String>,
    protocol_version: String,
}

#[derive(Default, Debug, Serialize)]
pub struct Amount {
    total: StringMinorUnit,
    currency: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    capture_flag: Option<bool>,
    reversal_reason_code: Option<String>,
    merchant_transaction_id: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDetails {
    merchant_id: Secret<String>,
    terminal_id: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInteraction {
    origin: TransactionInteractionOrigin,
    eci_indicator: TransactionInteractionEciIndicator,
    pos_condition_code: TransactionInteractionPosConditionCode,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionInteractionOrigin {
    #[default]
    Ecom,
}
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionInteractionEciIndicator {
    #[default]
    ChannelEncrypted,
}
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionInteractionPosConditionCode {
    #[default]
    CardNotPresentEcom,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ChaseorbitalCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ChaseOrbitalSessionObject {
    pub terminal_id: Secret<String>,
}

impl From<(&ChaseorbitalAuthType, &ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>)> for ChaseorbitalPaymentsRequest {
    fn from((auth, req): (&ChaseorbitalAuthType, &ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>)) -> Self {
        ChaseorbitalPaymentsRequest::try_from(req).expect("Failed to convert to ChaseorbitalPaymentsRequest")
    }
}
impl TryFrom<&ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>> for ChaseorbitalPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self,Self::Error> {

        let auth: ChaseorbitalAuthType = ChaseorbitalAuthType::try_from(&item.router_data.connector_auth_type)?;
        let amount = Amount {
            total: item.amount.clone(),
            currency: item.router_data.request.currency.to_string(),
        };
        let transaction_details = TransactionDetails {
            capture_flag: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            reversal_reason_code: None,
            merchant_transaction_id: item.router_data.connector_request_reference_id.clone(),
        };
        let metadata = "{\"terminal_id\" : \"001\"}".to_string(); //item.router_data.get_connector_meta()?;
        println!("metadata: {:?}", &metadata);

        let metadata = item.router_data.request.metadata.clone();
        // let session: ChaseOrbitalSessionObject = metadata
        //     .parse_value("ChaseOrbitalSessionObject")
        //     .change_context(errors::ConnectorError::InvalidConnectorConfig {
        //         config: "Merchant connector account metadata",
        //     })?;

        let merchant_details = MerchantDetails {
            merchant_id: auth.merchant_id,
            terminal_id: Some(Secret::new("session.terminal_id".to_string())),
        };
        let transaction_interaction = TransactionInteraction {
            //Payment is being made in online mode, card not present
            origin: TransactionInteractionOrigin::Ecom,
            // transaction encryption such as SSL/TLS, but authentication was not performed
            eci_indicator: TransactionInteractionEciIndicator::ChannelEncrypted,
            //card not present in online transaction
            pos_condition_code: TransactionInteractionPosConditionCode::CardNotPresentEcom,
        };
        let source = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ref ccard) => {
                let card = CardData {
                    card_data: ccard.card_number.clone(),
                    expiration_month: ccard.card_exp_month.clone(),
                    expiration_year: ccard.get_expiry_year_4_digit(),
                    security_code: ccard.card_cvc.clone(),
                };
                Source::PaymentCard { card }
            }
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("chaseorbital"),
                ))
            }?,
        };
        Ok(Self {
            amount,
            source,
            transaction_details,
            merchant_details,
            transaction_interaction,
        })

    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ChaseorbitalAuthType {
    pub(super) orb_user_name: Secret<String>,
    pub(super) orb_password: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ChaseorbitalAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                api_secret,
                key1
            } => Ok(Self {
                orb_user_name: api_key.to_owned(),
                orb_password: api_secret.to_owned(),
                merchant_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChaseorbitalPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ChaseorbitalPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: ChaseorbitalPaymentStatus) -> Self {
        match item {
            ChaseorbitalPaymentStatus::Succeeded => Self::Charged,
            ChaseorbitalPaymentStatus::Failed => Self::Failure,
            ChaseorbitalPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChaseorbitalPaymentsResponse {
    status: ChaseorbitalPaymentStatus,
    id: String,
}

impl<F,T> TryFrom<ResponseRouterData<F, ChaseorbitalPaymentsResponse, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ResponseRouterData<F, ChaseorbitalPaymentsResponse, T, PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ChaseorbitalRefundRequest {
    pub amount: StringMinorUnit
}

impl<F> TryFrom<&ChaseorbitalRouterData<&RefundsRouterData<F>>> for ChaseorbitalRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ChaseorbitalRouterData<&RefundsRouterData<F>>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: RefundsResponseRouterData<RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChaseorbitalErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
