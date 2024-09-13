pub mod transformers;

use base64::Engine;
use error_stack::{report, ResultExt};
use common_enums::enums;
use masking::{ExposeInterface, Mask, PeekInterface};

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
    request::{Method, Request, RequestBuilder, RequestContent},
};

use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData,
        PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorValidation},
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use router_env::logger;
use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils,
};

use transformers as chaseorbital;

#[derive(Clone)]
pub struct Chaseorbital {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync)
}

impl Chaseorbital {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector
        }
    }
}

impl api::Payment for Chaseorbital {}
impl api::PaymentSession for Chaseorbital {}
impl api::ConnectorAccessToken for Chaseorbital {}
impl api::MandateSetup for Chaseorbital {}
impl api::PaymentAuthorize for Chaseorbital {}
impl api::PaymentSync for Chaseorbital {}
impl api::PaymentCapture for Chaseorbital {}
impl api::PaymentVoid for Chaseorbital {}
impl api::Refund for Chaseorbital {}
impl api::RefundExecute for Chaseorbital {}
impl api::RefundSync for Chaseorbital {}
impl api::PaymentToken for Chaseorbital {}

impl
    ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Chaseorbital
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Chaseorbital
where
    Self: ConnectorIntegration<Flow, Request, Response>,{

    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        println!("header: {:?}", header);
        Ok(header)
    }
}

impl ConnectorCommon for Chaseorbital {
    fn id(&self) -> &'static str {
        "chaseorbital"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.chaseorbital.base_url.as_ref()
    }

    fn get_auth_header(&self, auth_type:&ConnectorAuthType)-> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        let auth =  chaseorbital::ChaseorbitalAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_key = format!("{}:{}", auth.orb_user_name.peek(), auth.orb_password.peek());
        dbg!(&auth_key);
        let auth_header = format!(
            "Basic {}",
            common_utils::consts::BASE64_ENGINE.encode(&auth_key)
        );

        Ok(vec![(headers::AUTHORIZATION.to_string(), auth_header.into_masked())])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: chaseorbital::ChaseorbitalErrorResponse = res
            .response
            .parse_struct("ChaseorbitalErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Chaseorbital
{
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl
    ConnectorIntegration<
        Session,
        PaymentsSessionData,
        PaymentsResponseData,
    > for Chaseorbital
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Chaseorbital
{
}

impl
    ConnectorIntegration<
        SetupMandate,
        SetupMandateRequestData,
        PaymentsResponseData,
    > for Chaseorbital
{
}

impl
    ConnectorIntegration<
        Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for Chaseorbital {
    fn get_headers(&self, req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &PaymentsAuthorizeRouterData, _connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Ok(format!(
            "{}authorize",
            _connectors.chaseorbital.base_url
        ))
    }

    fn get_request_body(&self, req: &PaymentsAuthorizeRouterData, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data =
            chaseorbital::ChaseorbitalRouterData::from((
                amount,
                req,
            ));
        let connector_req = chaseorbital::ChaseorbitalPaymentsRequest::try_from((&connector_router_data))?;
        logger::debug!("connector_req: {:?}", &connector_req);
        println!("connector_req: {:?}", &connector_req);
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let var = res.response;
        let response: chaseorbital::ChaseorbitalPaymentsResponse = var.parse_struct("Chaseorbital PaymentsAuthorizeResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
    for Chaseorbital
{
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: chaseorbital:: ChaseorbitalPaymentsResponse = res
            .response
            .parse_struct("chaseorbital PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        Capture,
        PaymentsCaptureData,
        PaymentsResponseData,
    > for Chaseorbital
{
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: chaseorbital::ChaseorbitalPaymentsResponse = res
            .response
            .parse_struct("Chaseorbital PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        Void,
        PaymentsCancelData,
        PaymentsResponseData,
    > for Chaseorbital
{}

impl
    ConnectorIntegration<
        Execute,
        RefundsData,
        RefundsResponseData,
    > for Chaseorbital {
    fn get_headers(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &RefundsRouterData<Execute>, _connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(&self, req: &RefundsRouterData<Execute>, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data =
            chaseorbital::ChaseorbitalRouterData::from((
                refund_amount,
                req,
            ));
        let connector_req = chaseorbital::ChaseorbitalRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<Option<Request>,errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(types::RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>,errors::ConnectorError> {
        let response: chaseorbital::RefundResponse = res.response.parse_struct("chaseorbital RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Chaseorbital {
    fn get_headers(&self, req: &RefundSyncRouterData,connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &RefundSyncRouterData,_connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData,errors::ConnectorError,> {
        let response: chaseorbital::RefundResponse = res.response.parse_struct("chaseorbital RefundSyncResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Chaseorbital {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
