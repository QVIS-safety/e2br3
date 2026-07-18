#[derive(Debug, Clone, Copy)]
pub struct EndpointPermissionContract {
	pub key: &'static str,
	pub method: &'static str,
	pub path: &'static str,
	pub permissions: &'static [&'static str],
}

pub const ENDPOINT_PERMISSION_CONTRACTS: &[EndpointPermissionContract] = &[
	EndpointPermissionContract {
		key: "case.create",
		method: "POST",
		path: "/api/cases",
		permissions: &["Case.Create", "SafetyReport.Create"],
	},
	EndpointPermissionContract {
		key: "case.update",
		method: "PUT",
		path: "/api/cases/{id}",
		permissions: &["Case.Update", "SafetyReport.Update"],
	},
	EndpointPermissionContract {
		key: "case.approve",
		method: "PUT",
		path: "/api/cases/{id}/workflow",
		permissions: &["Case.Approve"],
	},
	EndpointPermissionContract {
		key: "info.create",
		method: "POST",
		path: "/api/presaves/{section}",
		permissions: &["PresaveTemplate.Create"],
	},
	EndpointPermissionContract {
		key: "info.update",
		method: "PUT",
		path: "/api/presaves/{section}/{id}",
		permissions: &["PresaveTemplate.Update"],
	},
	EndpointPermissionContract {
		key: "info.delete",
		method: "DELETE",
		path: "/api/presaves/{section}/{id}",
		permissions: &["PresaveTemplate.Delete"],
	},
	EndpointPermissionContract {
		key: "import.history",
		method: "GET",
		path: "/api/import/history",
		permissions: &["XmlImport.Read"],
	},
	EndpointPermissionContract {
		key: "import.execute",
		method: "POST",
		path: "/api/import",
		permissions: &["XmlImport.Import"],
	},
	EndpointPermissionContract {
		key: "submission.history",
		method: "GET",
		path: "/api/export/history",
		permissions: &["XmlExport.Read"],
	},
	EndpointPermissionContract {
		key: "submission.execute",
		method: "POST",
		path: "/api/export",
		permissions: &["XmlExport.Export"],
	},
	EndpointPermissionContract {
		key: "settings.read",
		method: "GET",
		path: "/api/admin/settings",
		permissions: &["Settings.Read"],
	},
	EndpointPermissionContract {
		key: "settings.update",
		method: "PUT",
		path: "/api/admin/settings",
		permissions: &["Settings.Update"],
	},
	EndpointPermissionContract {
		key: "users.list",
		method: "GET",
		path: "/api/users",
		permissions: &["User.List"],
	},
	EndpointPermissionContract {
		key: "users.create",
		method: "POST",
		path: "/api/users",
		permissions: &["User.Create"],
	},
	EndpointPermissionContract {
		key: "users.update",
		method: "PUT",
		path: "/api/users/{id}",
		permissions: &["User.Update"],
	},
	EndpointPermissionContract {
		key: "users.delete",
		method: "DELETE",
		path: "/api/users/{id}",
		permissions: &["User.Delete"],
	},
	EndpointPermissionContract {
		key: "organization.update",
		method: "PUT",
		path: "/api/organizations/{id}",
		permissions: &["Organization.Update"],
	},
	EndpointPermissionContract {
		key: "terminology.read",
		method: "GET",
		path: "/api/terminology",
		permissions: &["Terminology.Read"],
	},
	EndpointPermissionContract {
		key: "dashboard.notice.update",
		method: "PUT",
		path: "/api/admin/settings/notices",
		permissions: &["DashboardNotice.Update"],
	},
];
