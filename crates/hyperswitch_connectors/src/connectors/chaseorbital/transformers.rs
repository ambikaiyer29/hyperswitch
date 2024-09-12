use common_enums::enums;
use serde::{Deserialize, Serialize};
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
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
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
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ChaseorbitalPaymentsRequest {
    amount: StringMinorUnit,
    // card: ChaseorbitalCard,
    orbitalConnectionUserName: Secret<String>,
    orbitalConnectionPassword: Secret<String>,
    industryType: String,
    messageType: String,
    bin: String,
    merchantID: Secret<String>,
    terminalID: Secret<String>,
    accountNum: String,
    exp: String,
    currencyCode: String,
    currencyExponent: String,
    avsZip:String,
    avsAddress1: String,
    orderId: String,
    cardIndicators: String,

}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ChaseorbitalCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl From<(&ChaseorbitalAuthType, &ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>)> for ChaseorbitalPaymentsRequest {
    fn from((auth, req): (&ChaseorbitalAuthType, &ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>)) -> Self {

        let mut connector_req: ChaseorbitalPaymentsRequest = ChaseorbitalPaymentsRequest::try_from(req).expect("Failed to convert to ChaseorbitalPaymentsRequest");

        connector_req.orbitalConnectionUserName = auth.orb_user_name.clone();
        connector_req.orbitalConnectionPassword = auth.orb_password.clone();
        connector_req.merchantID = auth.merchant_id.clone();

        connector_req
    }
}
impl TryFrom<&ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>> for ChaseorbitalPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ChaseorbitalRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self,Self::Error> {
     let source_var =    match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {

                let card = ChaseorbitalCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok::<ChaseorbitalPaymentsRequest, Self::Error>(Self {
                    amount: item.amount.clone(),
                    orbitalConnectionUserName: Default::default(),
                    orbitalConnectionPassword: Default::default(),
                    industryType: "EC".to_string(),
                    messageType: "AC".to_string(),
                    bin: "000001".to_string(),
                    merchantID: Secret::new("253997".to_string()),
                    terminalID: Secret::new("001".to_string()),
                    accountNum: "4000000000000002".to_string(),
                    exp: "0624".to_string(),
                    currencyCode: "124".to_string(),
                    currencyExponent: "2".to_string(),
                    avsZip: "94043".to_string(),
                    avsAddress1: "Charleston Road".to_string(),
                    orderId: "P-00000079".to_string(),
                    cardIndicators: "Y".to_string(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }?;

        Ok(source_var)
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
