/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub(crate) mod connection {
    use std::time::Instant;

    use crate::service::ConnectionID;

    pub(crate) fn connection_open_res(
        connection_id: ConnectionID,
        receive_time: Instant,
        databases_all_res: typedb_protocol::database_manager::all::Res,
    ) -> typedb_protocol::connection::open::Res {
        let processing_millis = Instant::now().duration_since(receive_time).as_millis();
        typedb_protocol::connection::open::Res {
            connection_id: Some(typedb_protocol::ConnectionId { id: Vec::from(connection_id) }),
            server_duration_millis: processing_millis as u64,
            databases_all: Some(databases_all_res),
        }
    }
}

pub(crate) mod server_manager {
    use std::net::SocketAddr;

    pub(crate) fn servers_all_res(address: &SocketAddr) -> typedb_protocol::server_manager::all::Res {
        typedb_protocol::server_manager::all::Res {
            servers: vec![typedb_protocol::Server { address: address.to_string() }],
        }
    }
}

pub(crate) mod database_manager {
    use std::net::SocketAddr;

    pub(crate) fn database_get_res(
        server_address: &SocketAddr,
        database_name: String,
    ) -> typedb_protocol::database_manager::get::Res {
        typedb_protocol::database_manager::get::Res {
            database: Some(typedb_protocol::DatabaseReplicas {
                name: database_name,
                replicas: Vec::from([typedb_protocol::database_replicas::Replica {
                    address: server_address.to_string(),
                    primary: true,
                    preferred: true,
                    term: 0,
                }]),
            }),
        }
    }

    pub(crate) fn database_all_res(
        server_address: &SocketAddr,
        database_names: Vec<String>,
    ) -> typedb_protocol::database_manager::all::Res {
        typedb_protocol::database_manager::all::Res {
            databases: database_names.into_iter().map(|name| database_replicas(name, server_address)).collect(),
        }
    }

    pub(crate) fn database_contains_res(contains: bool) -> typedb_protocol::database_manager::contains::Res {
        typedb_protocol::database_manager::contains::Res { contains }
    }

    pub(crate) fn database_replicas(name: String, server_address: &SocketAddr) -> typedb_protocol::DatabaseReplicas {
        typedb_protocol::DatabaseReplicas {
            name: name,
            replicas: Vec::from([typedb_protocol::database_replicas::Replica {
                address: server_address.to_string(),
                primary: true,
                preferred: true,
                term: 0,
            }]),
        }
    }

    pub(crate) fn database_create_res(
        name: String,
        server_address: &SocketAddr,
    ) -> typedb_protocol::database_manager::create::Res {
        typedb_protocol::database_manager::create::Res { database: Some(database_replicas(name, server_address)) }
    }
}

pub(crate) mod database {
    pub(crate) fn database_delete_res() -> typedb_protocol::database::delete::Res {
        typedb_protocol::database::delete::Res {}
    }
}

pub(crate) mod transaction {
    use uuid::Uuid;

    pub(crate) fn transaction_open_res(
        req_id: Uuid,
        server_processing_millis: u64,
    ) -> typedb_protocol::transaction::Server {
        let message = typedb_protocol::transaction::res::Res::OpenRes(typedb_protocol::transaction::open::Res {
            server_duration_millis: server_processing_millis,
        });
        transaction_server_res(req_id, message)
    }

    pub(crate) fn query_res_ok_empty() -> typedb_protocol::query::initial_res::ok::Ok {
        typedb_protocol::query::initial_res::ok::Ok::Empty(typedb_protocol::query::initial_res::ok::Empty {})
    }

    pub(crate) fn query_res_ok_concept_row_stream(
        column_variable_names: Vec<String>,
    ) -> typedb_protocol::query::initial_res::ok::Ok {
        typedb_protocol::query::initial_res::ok::Ok::ConceptRowStream(
            typedb_protocol::query::initial_res::ok::ConceptRowStream { column_variable_names },
        )
    }

    pub(crate) fn query_res_ok_readable_concept_tree_stream() -> typedb_protocol::query::initial_res::ok::Ok {
        typedb_protocol::query::initial_res::ok::Ok::ReadableConceptTreeStream(
            typedb_protocol::query::initial_res::ok::ReadableConceptTreeStream {},
        )
    }

    pub(crate) fn query_initial_res_ok_from_query_res_ok_ok(
        message: typedb_protocol::query::initial_res::ok::Ok,
    ) -> typedb_protocol::query::initial_res::Ok {
        typedb_protocol::query::initial_res::Ok { ok: Some(message) }
    }

    pub(crate) fn query_initial_res_from_query_res_ok(
        message: typedb_protocol::query::initial_res::Ok,
    ) -> typedb_protocol::query::InitialRes {
        typedb_protocol::query::InitialRes { res: Some(typedb_protocol::query::initial_res::Res::Ok(message)) }
    }

    pub(crate) fn query_initial_res_from_error(error: typedb_protocol::Error) -> typedb_protocol::query::InitialRes {
        typedb_protocol::query::InitialRes { res: Some(typedb_protocol::query::initial_res::Res::Error(error)) }
    }

    pub(crate) fn query_res_part_from_concept_rows(
        messages: Vec<typedb_protocol::ConceptRow>,
    ) -> typedb_protocol::query::ResPart {
        typedb_protocol::query::ResPart {
            res: Some(typedb_protocol::query::res_part::Res::RowsRes(
                typedb_protocol::query::res_part::ConceptRowsRes { rows: messages },
            )),
        }
    }

    pub(crate) fn query_res_part_from_concept_tree() {
        todo!()
    }

    // -----------

    #[inline]
    fn transaction_res_part_res_part_stream_signal_done() -> typedb_protocol::transaction::res_part::ResPart {
        typedb_protocol::transaction::res_part::ResPart::StreamRes(
            typedb_protocol::transaction::stream_signal::ResPart {
                state: Some(typedb_protocol::transaction::stream_signal::res_part::State::Done(
                    typedb_protocol::transaction::stream_signal::res_part::Done {},
                )),
            },
        )
    }

    #[inline]
    fn transaction_res_part_res_part_stream_signal_continue() -> typedb_protocol::transaction::res_part::ResPart {
        typedb_protocol::transaction::res_part::ResPart::StreamRes(
            typedb_protocol::transaction::stream_signal::ResPart {
                state: Some(typedb_protocol::transaction::stream_signal::res_part::State::Continue(
                    typedb_protocol::transaction::stream_signal::res_part::Continue {},
                )),
            },
        )
    }

    #[inline]
    fn transaction_res_part_res_part_stream_signal_error(
        error_message: typedb_protocol::Error,
    ) -> typedb_protocol::transaction::res_part::ResPart {
        typedb_protocol::transaction::res_part::ResPart::StreamRes(
            typedb_protocol::transaction::stream_signal::ResPart {
                state: Some(typedb_protocol::transaction::stream_signal::res_part::State::Error(error_message)),
            },
        )
    }

    #[inline]
    fn transaction_server_res(
        req_id: Uuid,
        message: typedb_protocol::transaction::res::Res,
    ) -> typedb_protocol::transaction::Server {
        typedb_protocol::transaction::Server {
            server: Some(typedb_protocol::transaction::server::Server::Res(typedb_protocol::transaction::Res {
                req_id: req_id.as_bytes().to_vec(),
                res: Some(message),
            })),
        }
    }

    #[inline]
    fn transaction_server_res_part(
        req_id: Uuid,
        message: typedb_protocol::transaction::res_part::ResPart,
    ) -> typedb_protocol::transaction::Server {
        typedb_protocol::transaction::Server {
            server: Some(typedb_protocol::transaction::server::Server::ResPart(
                typedb_protocol::transaction::ResPart { req_id: req_id.as_bytes().to_vec(), res_part: Some(message) },
            )),
        }
    }

    // helpers

    pub(crate) fn transaction_server_res_query_res(
        req_id: Uuid,
        message: typedb_protocol::query::InitialRes,
    ) -> typedb_protocol::transaction::Server {
        transaction_server_res(req_id, typedb_protocol::transaction::res::Res::QueryInitialRes(message))
    }

    #[inline]
    pub(crate) fn transaction_server_res_parts_query_part(
        req_id: Uuid,
        res_part: typedb_protocol::query::ResPart,
    ) -> typedb_protocol::transaction::Server {
        transaction_server_res_part(req_id, typedb_protocol::transaction::res_part::ResPart::QueryRes(res_part))
    }

    #[inline]
    pub(crate) fn transaction_server_res_part_stream_signal_continue(
        req_id: Uuid,
    ) -> typedb_protocol::transaction::Server {
        transaction_server_res_part(req_id, transaction_res_part_res_part_stream_signal_continue())
    }

    #[inline]
    pub(crate) fn transaction_server_res_part_stream_signal_done(req_id: Uuid) -> typedb_protocol::transaction::Server {
        transaction_server_res_part(req_id, transaction_res_part_res_part_stream_signal_done())
    }

    #[inline]
    pub(crate) fn transaction_server_res_part_stream_signal_error(
        req_id: Uuid,
        error: typedb_protocol::Error,
    ) -> typedb_protocol::transaction::Server {
        transaction_server_res_part(req_id, transaction_res_part_res_part_stream_signal_error(error))
    }
}