#![allow(dead_code)]

/// Request message for `SetIamPolicy` method.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SetIamPolicyRequest {
    /// REQUIRED: The resource for which the policy is being specified.
    /// See the operation documentation for the appropriate value for this field.
    pub resource: String,

    /// REQUIRED: The complete policy to be applied to the `resource`. The size of
    /// the policy is limited to a few 10s of KB. An empty policy is a
    /// valid policy but certain Cloud Platform services (such as Projects)
    /// might reject them.
    pub policy: Option<crate::model::Policy>,

    /// OPTIONAL: A FieldMask specifying which fields of the policy to modify. Only
    /// the fields in the mask will be modified. If no mask is provided, the
    /// following default mask is used:
    ///
    /// `paths: "bindings, etag"`
    pub update_mask: Option<gax_placeholder::FieldMask>,
}

/// Request message for `GetIamPolicy` method.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct GetIamPolicyRequest {
    /// REQUIRED: The resource for which the policy is being requested.
    /// See the operation documentation for the appropriate value for this field.
    pub resource: String,

    /// OPTIONAL: A `GetPolicyOptions` object for specifying options to
    /// `GetIamPolicy`.
    pub options: Option<crate::model::GetPolicyOptions>,
}

/// Request message for `TestIamPermissions` method.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TestIamPermissionsRequest {
    /// REQUIRED: The resource for which the policy detail is being requested.
    /// See the operation documentation for the appropriate value for this field.
    pub resource: String,

    /// The set of permissions to check for the `resource`. Permissions with
    /// wildcards (such as '*' or 'storage.*') are not allowed. For more
    /// information see
    /// [IAM Overview](https://cloud.google.com/iam/docs/overview#permissions).
    pub permissions: Vec<String>,
}

/// Response message for `TestIamPermissions` method.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TestIamPermissionsResponse {
    /// A subset of `TestPermissionsRequest.permissions` that the caller is
    /// allowed.
    pub permissions: Vec<String>,
}

/// Encapsulates settings provided to GetIamPolicy.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct GetPolicyOptions {
    /// Optional. The maximum policy version that will be used to format the
    /// policy.
    ///
    /// Valid values are 0, 1, and 3. Requests specifying an invalid value will be
    /// rejected.
    ///
    /// Requests for policies with any conditional role bindings must specify
    /// version 3. Policies with no conditional role bindings may specify any valid
    /// value or leave the field unset.
    ///
    /// The policy in the response might use the policy version that you specified,
    /// or it might use a lower policy version. For example, if you specify version
    /// 3, but the policy has no conditional role bindings, the response uses
    /// version 1.
    ///
    /// To learn which resources support conditions in their IAM policies, see the
    /// [IAM
    /// documentation](https://cloud.google.com/iam/help/conditions/resource-policies).
    pub requested_policy_version: i32,
}

/// An Identity and Access Management (IAM) policy, which specifies access
/// controls for Google Cloud resources.
///
///
/// A `Policy` is a collection of `bindings`. A `binding` binds one or more
/// `members`, or principals, to a single `role`. Principals can be user
/// accounts, service accounts, Google groups, and domains (such as G Suite). A
/// `role` is a named list of permissions; each `role` can be an IAM predefined
/// role or a user-created custom role.
///
/// For some types of Google Cloud resources, a `binding` can also specify a
/// `condition`, which is a logical expression that allows access to a resource
/// only if the expression evaluates to `true`. A condition can add constraints
/// based on attributes of the request, the resource, or both. To learn which
/// resources support conditions in their IAM policies, see the
/// [IAM
/// documentation](https://cloud.google.com/iam/help/conditions/resource-policies).
///
/// **JSON example:**
///
/// ```norust
///     {
///       "bindings": [
///         {
///           "role": "roles/resourcemanager.organizationAdmin",
///           "members": [
///             "user:mike@example.com",
///             "group:admins@example.com",
///             "domain:google.com",
///             "serviceAccount:my-project-id@appspot.gserviceaccount.com"
///           ]
///         },
///         {
///           "role": "roles/resourcemanager.organizationViewer",
///           "members": [
///             "user:eve@example.com"
///           ],
///           "condition": {
///             "title": "expirable access",
///             "description": "Does not grant access after Sep 2020",
///             "expression": "request.time <
///             timestamp('2020-10-01T00:00:00.000Z')",
///           }
///         }
///       ],
///       "etag": "BwWWja0YfJA=",
///       "version": 3
///     }
/// ```
///
/// **YAML example:**
///
/// ```norust
///     bindings:
///     - members:
///       - user:mike@example.com
///       - group:admins@example.com
///       - domain:google.com
///       - serviceAccount:my-project-id@appspot.gserviceaccount.com
///       role: roles/resourcemanager.organizationAdmin
///     - members:
///       - user:eve@example.com
///       role: roles/resourcemanager.organizationViewer
///       condition:
///         title: expirable access
///         description: Does not grant access after Sep 2020
///         expression: request.time < timestamp('2020-10-01T00:00:00.000Z')
///     etag: BwWWja0YfJA=
///     version: 3
/// ```
///
/// For a description of IAM and its features, see the
/// [IAM documentation](https://cloud.google.com/iam/docs/).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Policy {
    /// Specifies the format of the policy.
    ///
    /// Valid values are `0`, `1`, and `3`. Requests that specify an invalid value
    /// are rejected.
    ///
    /// Any operation that affects conditional role bindings must specify version
    /// `3`. This requirement applies to the following operations:
    ///
    /// * Getting a policy that includes a conditional role binding
    /// * Adding a conditional role binding to a policy
    /// * Changing a conditional role binding in a policy
    /// * Removing any role binding, with or without a condition, from a policy
    ///   that includes conditions
    ///
    /// **Important:** If you use IAM Conditions, you must include the `etag` field
    /// whenever you call `setIamPolicy`. If you omit this field, then IAM allows
    /// you to overwrite a version `3` policy with a version `1` policy, and all of
    /// the conditions in the version `3` policy are lost.
    ///
    /// If a policy does not include any conditions, operations on that policy may
    /// specify any valid version or leave the field unset.
    ///
    /// To learn which resources support conditions in their IAM policies, see the
    /// [IAM
    /// documentation](https://cloud.google.com/iam/help/conditions/resource-policies).
    pub version: i32,

    /// Associates a list of `members`, or principals, with a `role`. Optionally,
    /// may specify a `condition` that determines how and when the `bindings` are
    /// applied. Each of the `bindings` must contain at least one principal.
    ///
    /// The `bindings` in a `Policy` can refer to up to 1,500 principals; up to 250
    /// of these principals can be Google groups. Each occurrence of a principal
    /// counts towards these limits. For example, if the `bindings` grant 50
    /// different roles to `user:alice@example.com`, and not to any other
    /// principal, then you can add another 1,450 principals to the `bindings` in
    /// the `Policy`.
    pub bindings: Vec<crate::model::Binding>,

    /// Specifies cloud audit logging configuration for this policy.
    pub audit_configs: Vec<crate::model::AuditConfig>,

    /// `etag` is used for optimistic concurrency control as a way to help
    /// prevent simultaneous updates of a policy from overwriting each other.
    /// It is strongly suggested that systems make use of the `etag` in the
    /// read-modify-write cycle to perform policy updates in order to avoid race
    /// conditions: An `etag` is returned in the response to `getIamPolicy`, and
    /// systems are expected to put that etag in the request to `setIamPolicy` to
    /// ensure that their change will be applied to the same version of the policy.
    ///
    /// **Important:** If you use IAM Conditions, you must include the `etag` field
    /// whenever you call `setIamPolicy`. If you omit this field, then IAM allows
    /// you to overwrite a version `3` policy with a version `1` policy, and all of
    /// the conditions in the version `3` policy are lost.
    pub etag: bytes::Bytes,
}

/// Associates `members`, or principals, with a `role`.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Binding {
    /// Role that is assigned to the list of `members`, or principals.
    /// For example, `roles/viewer`, `roles/editor`, or `roles/owner`.
    pub role: String,

    /// Specifies the principals requesting access for a Google Cloud resource.
    /// `members` can have the following values:
    ///
    /// * `allUsers`: A special identifier that represents anyone who is
    ///    on the internet; with or without a Google account.
    ///
    /// * `allAuthenticatedUsers`: A special identifier that represents anyone
    ///    who is authenticated with a Google account or a service account.
    ///
    /// * `user:{emailid}`: An email address that represents a specific Google
    ///    account. For example, `alice@example.com` .
    ///
    ///
    /// * `serviceAccount:{emailid}`: An email address that represents a service
    ///    account. For example, `my-other-app@appspot.gserviceaccount.com`.
    ///
    /// * `group:{emailid}`: An email address that represents a Google group.
    ///    For example, `admins@example.com`.
    ///
    /// * `deleted:user:{emailid}?uid={uniqueid}`: An email address (plus unique
    ///    identifier) representing a user that has been recently deleted. For
    ///    example, `alice@example.com?uid=123456789012345678901`. If the user is
    ///    recovered, this value reverts to `user:{emailid}` and the recovered user
    ///    retains the role in the binding.
    ///
    /// * `deleted:serviceAccount:{emailid}?uid={uniqueid}`: An email address (plus
    ///    unique identifier) representing a service account that has been recently
    ///    deleted. For example,
    ///    `my-other-app@appspot.gserviceaccount.com?uid=123456789012345678901`.
    ///    If the service account is undeleted, this value reverts to
    ///    `serviceAccount:{emailid}` and the undeleted service account retains the
    ///    role in the binding.
    ///
    /// * `deleted:group:{emailid}?uid={uniqueid}`: An email address (plus unique
    ///    identifier) representing a Google group that has been recently
    ///    deleted. For example, `admins@example.com?uid=123456789012345678901`. If
    ///    the group is recovered, this value reverts to `group:{emailid}` and the
    ///    recovered group retains the role in the binding.
    ///
    ///
    /// * `domain:{domain}`: The G Suite domain (primary) that represents all the
    ///    users of that domain. For example, `google.com` or `example.com`.
    ///
    ///
    pub members: Vec<String>,

    /// The condition that is associated with this binding.
    ///
    /// If the condition evaluates to `true`, then this binding applies to the
    /// current request.
    ///
    /// If the condition evaluates to `false`, then this binding does not apply to
    /// the current request. However, a different role binding might grant the same
    /// role to one or more of the principals in this binding.
    ///
    /// To learn which resources support conditions in their IAM policies, see the
    /// [IAM
    /// documentation](https://cloud.google.com/iam/help/conditions/resource-policies).
    pub condition: Option<gtype::model::Expr>,
}

/// Specifies the audit configuration for a service.
/// The configuration determines which permission types are logged, and what
/// identities, if any, are exempted from logging.
/// An AuditConfig must have one or more AuditLogConfigs.
///
/// If there are AuditConfigs for both `allServices` and a specific service,
/// the union of the two AuditConfigs is used for that service: the log_types
/// specified in each AuditConfig are enabled, and the exempted_members in each
/// AuditLogConfig are exempted.
///
/// Example Policy with multiple AuditConfigs:
///
///     {
///       "audit_configs": [
///         {
///           "service": "allServices",
///           "audit_log_configs": [
///             {
///               "log_type": "DATA_READ",
///               "exempted_members": [
///                 "user:jose@example.com"
///               ]
///             },
///             {
///               "log_type": "DATA_WRITE"
///             },
///             {
///               "log_type": "ADMIN_READ"
///             }
///           ]
///         },
///         {
///           "service": "sampleservice.googleapis.com",
///           "audit_log_configs": [
///             {
///               "log_type": "DATA_READ"
///             },
///             {
///               "log_type": "DATA_WRITE",
///               "exempted_members": [
///                 "user:aliya@example.com"
///               ]
///             }
///           ]
///         }
///       ]
///     }
///
/// For sampleservice, this policy enables DATA_READ, DATA_WRITE and ADMIN_READ
/// logging. It also exempts `jose@example.com` from DATA_READ logging, and
/// `aliya@example.com` from DATA_WRITE logging.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AuditConfig {
    /// Specifies a service that will be enabled for audit logging.
    /// For example, `storage.googleapis.com`, `cloudsql.googleapis.com`.
    /// `allServices` is a special value that covers all services.
    pub service: String,

    /// The configuration for logging of each type of permission.
    pub audit_log_configs: Vec<crate::model::AuditLogConfig>,
}

/// Provides the configuration for logging a type of permissions.
/// Example:
///
///     {
///       "audit_log_configs": [
///         {
///           "log_type": "DATA_READ",
///           "exempted_members": [
///             "user:jose@example.com"
///           ]
///         },
///         {
///           "log_type": "DATA_WRITE"
///         }
///       ]
///     }
///
/// This enables 'DATA_READ' and 'DATA_WRITE' logging, while exempting
/// jose@example.com from DATA_READ logging.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AuditLogConfig {
    /// The log type that this config enables.
    pub log_type: crate::model::audit_log_config::LogType,

    /// Specifies the identities that do not cause logging for this type of
    /// permission.
    /// Follows the same format of
    /// [Binding.members][google.iam.v1.Binding.members].
    pub exempted_members: Vec<String>,
}

/// Defines additional types related to AuditLogConfig
pub mod audit_log_config {

    /// The list of valid permission types for which logging can be configured.
    /// Admin writes are always logged, and are not configurable.
    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct LogType(String);

    /// Useful constants to work with [LogType](LogType)
    pub mod log_type {

        /// Default case. Should never be this.
        pub const LOG_TYPE_UNSPECIFIED: &str = "LOG_TYPE_UNSPECIFIED";

        /// Admin reads. Example: CloudIAM getIamPolicy
        pub const ADMIN_READ: &str = "ADMIN_READ";

        /// Data writes. Example: CloudSQL Users create
        pub const DATA_WRITE: &str = "DATA_WRITE";

        /// Data reads. Example: CloudSQL Users list
        pub const DATA_READ: &str = "DATA_READ";
    }
}

/// The difference delta between two policies.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PolicyDelta {
    /// The delta for Bindings between two policies.
    pub binding_deltas: Vec<crate::model::BindingDelta>,

    /// The delta for AuditConfigs between two policies.
    pub audit_config_deltas: Vec<crate::model::AuditConfigDelta>,
}

/// One delta entry for Binding. Each individual change (only one member in each
/// entry) to a binding will be a separate entry.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BindingDelta {
    /// The action that was performed on a Binding.
    /// Required
    pub action: crate::model::binding_delta::Action,

    /// Role that is assigned to `members`.
    /// For example, `roles/viewer`, `roles/editor`, or `roles/owner`.
    /// Required
    pub role: String,

    /// A single identity requesting access for a Google Cloud resource.
    /// Follows the same format of Binding.members.
    /// Required
    pub member: String,

    /// The condition that is associated with this binding.
    pub condition: Option<gtype::model::Expr>,
}

/// Defines additional types related to BindingDelta
pub mod binding_delta {

    /// The type of action performed on a Binding in a policy.
    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct Action(String);

    /// Useful constants to work with [Action](Action)
    pub mod action {

        /// Unspecified.
        pub const ACTION_UNSPECIFIED: &str = "ACTION_UNSPECIFIED";

        /// Addition of a Binding.
        pub const ADD: &str = "ADD";

        /// Removal of a Binding.
        pub const REMOVE: &str = "REMOVE";
    }
}

/// One delta entry for AuditConfig. Each individual change (only one
/// exempted_member in each entry) to a AuditConfig will be a separate entry.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AuditConfigDelta {
    /// The action that was performed on an audit configuration in a policy.
    /// Required
    pub action: crate::model::audit_config_delta::Action,

    /// Specifies a service that was configured for Cloud Audit Logging.
    /// For example, `storage.googleapis.com`, `cloudsql.googleapis.com`.
    /// `allServices` is a special value that covers all services.
    /// Required
    pub service: String,

    /// A single identity that is exempted from "data access" audit
    /// logging for the `service` specified above.
    /// Follows the same format of Binding.members.
    pub exempted_member: String,

    /// Specifies the log_type that was be enabled. ADMIN_ACTIVITY is always
    /// enabled, and cannot be configured.
    /// Required
    pub log_type: String,
}

/// Defines additional types related to AuditConfigDelta
pub mod audit_config_delta {

    /// The type of action performed on an audit configuration in a policy.
    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    pub struct Action(String);

    /// Useful constants to work with [Action](Action)
    pub mod action {

        /// Unspecified.
        pub const ACTION_UNSPECIFIED: &str = "ACTION_UNSPECIFIED";

        /// Addition of an audit configuration.
        pub const ADD: &str = "ADD";

        /// Removal of an audit configuration.
        pub const REMOVE: &str = "REMOVE";
    }
}