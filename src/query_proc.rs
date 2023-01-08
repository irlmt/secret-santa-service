use std::string::String;
use serde_json::{Map, Value};

use crate::defs;
use crate::state::*;
use crate::auth;
use crate::proc_for_users as gr_usr;
use crate::proc_for_admin as gr_adm;

pub(crate) fn process_req(req: Map<String, Value>, data_state: &mut DataState) -> (String, String) {
    let LACKING_FIELDS: (String, String) = (defs::HTTP_STATUS_400.to_string(), "{\"error\": \"Не все обязательные поля заполнены значениями требуемых форматов\"}".to_string());
    let UNSUPPORTED_OP: (String, String) = (defs::HTTP_STATUS_400.to_string(), "{\"error\":\"Неподдерживаемая, невыполнимая или неверно сформированная операция\"}".to_string());
    let INTERNAL_ERROR: (String, String) = (defs::HTTP_STATUS_500.to_string(), "{\"error\":\"Не удалось выполнить операцию из-за неполадок на сервере\"}".to_string());
    let NOT_FOUND: (String, String) = (defs::HTTP_STATUS_404.to_string(), "Не удалось найти информацию о запрашиваемом объекте".to_string());
    // Перед считыванием значения по ключу обязательно проверять, что пара с желаемым ключом есть в структуре, чтобы не было вылета
    /* // шаблонные примеры - как считывать значения пар "ключ:значение"
    if req.contains_key("sat") {
        println!("sat: {}", req["sat"]);
    }
    if req.contains_key("action") {
        println!("action: {}", req["action"]);
    }*/
    // По результатам чтения и обработки req_map и выполнения в результате каких-то действий - возвращаем две строки:
    //     status_line (заголовок - статус ответа HTTP) и contents (тело ответа)
    let (mut s, mut c): (String, String) = ("".to_string(), "".to_string()); // статус HTTP-ответа; тело HTTP-ответа
    if req.contains_key("action") {
        // операции, требующие авторизацию (sat - токен)
        let UNAUTHORIZED_OP = (defs::HTTP_STATUS_401.to_string(), "{\"error\":\"Для данной операции требуется авторизация в системе\"}".to_string());
        if req.contains_key("sat") {
            if req["sat"].is_string() {
                let uid: u64 = auth::verify_auth(req["sat"].as_str().unwrap().to_string(), data_state);
                if uid == 0 {
                    (s, c) = UNAUTHORIZED_OP;
                } else {
                    (s, c) = match req["action"].as_str() {
                        Some("new_group") => {
                            match req.contains_key("name") && req["name"].is_string() {
                                true => {
                                    let gid = gr_usr::make_group(uid, req["name"].as_str().unwrap().to_string(), data_state);
                                    match gid {
                                        0 => INTERNAL_ERROR,
                                        _ => (defs::HTTP_STATUS_200.to_string(), "{\"id\":".to_string() + &gid.to_string() + "}")
                                    }
                                },
                                false => LACKING_FIELDS
                            }
                        },
                        Some("logout") => {
                            match auth::logout(req["sat"].as_str().unwrap().to_string(), data_state) {
                                true => (defs::HTTP_STATUS_200.to_string(), "{\"msg\":\"Сеанс завершен\"}".to_string()),
                                false => (defs::HTTP_STATUS_500.to_string(), "{\"error\": \"Не найден сеанс для завершения (ошибка сервера)\"}".to_string())
                            }
                        },
                        //Some("")
                        _ => UNSUPPORTED_OP
                    };
                }
            } else {
                (s, c) = UNAUTHORIZED_OP;
            }
        } else {
            // операции, не требующие авторизацию
            (s, c) = match req["action"].as_str() {
                Some("login") => {
                    /* проверить логин и пароль
                    //       должно быть наличие id пользователя req["uid"] и пароля req["password"]
                    //       uid - число, password - непустая строка
                    //       и должна быть в бд/файле запись в таблице User с парой id-пароль, совпадающей с переданными uid и password
                    //       (для пароля проверяется совпадение хэша переданной строки password с хранимым хэшем password в записи в таблице User)
                    //       Если что-то не существует или не в соответствующем формате, то
                    //return (defs::HTTP_STATUS_401.to_string(), "Неверный или некорректно сформированный id пользователя и/или пароль".to_string());
                    //       Иначе (если все ок) - сформировать уникальный токен
                    //                             создать новую запись в таблице Auth с переданным uid и сформированным токеном token
                    //                             отдать token клиенту
                    //return (defs::HTTP_STATUS_200.to_string(), token); */
                    match req.contains_key("uid") && req.contains_key("password") &&
                          req["uid"].is_u64() && req["password"].is_string() {
                        true => {
                            let uid = req["uid"].as_u64().unwrap();
                            let pw = req["password"].as_str().unwrap().to_string();
                            let token = auth::auth(uid, pw, data_state);
                            match token.is_empty() {
                                true => (defs::HTTP_STATUS_401.to_string(), "{\"error\":\"Неверный или некорректно сформированный id пользователя и/или пароль\"}".to_string()),
                                false => (defs::HTTP_STATUS_200.to_string(), "{\"token\":\"".to_string() + &token + "\"}")
                            }
                        },
                        false => LACKING_FIELDS
                    }
                },
                Some("reg") => {
                    match req.contains_key("name") && req.contains_key("password") && req.contains_key("password_repeat") && 
                          req["name"].is_string() && req["password"].is_string() && req["password_repeat"].is_string() {
                        true => {
                            let name = req["name"].as_str().unwrap().to_string();
                            let password = req["password"].as_str().unwrap().to_string();
                            let password_repeat = req["password_repeat"].as_str().unwrap().to_string();
                            let uid = auth::reg(name, password, password_repeat, data_state);
                            match uid {
                                0 => (defs::HTTP_STATUS_400.to_string(), "{\"error\":\"Неверные значения полей, либо не совпадают пароль и повтор пароля\"}".to_string()),
                                _ => (defs::HTTP_STATUS_200.to_string(), "{\"id\":".to_string() + &uid.to_string() + "}")
                            }
                        },
                        false => LACKING_FIELDS
                    }
                },
                // TODO:
                Some("group_stat") => {
                    match req.contains_key("gid") && req["gid"].is_u64() && req["gid"].as_u64().unwrap() > 0 {
                        true => {
                            let mut exists: bool = false;
                            let gid = req["gid"].as_u64().unwrap();
                            let mut group_name: String = "".to_string();
                            let mut group_is_closed: bool = false;
                            {
                                let data_state_group_ref = &data_state.group;
                                for rec in data_state_group_ref {
                                    if rec.id == gid {
                                        exists = true;
                                        group_name = rec.name.clone();
                                        group_is_closed = rec.is_closed;
                                        break;
                                    }
                                }
                            }
                            match exists {
                                true => {
                                    let urecs = gr_usr::list_users_in_group_recs(gid, data_state);
                                    let mut resp_arr: String = "{\"name\":\"".to_string() + &group_name +
                                        "\", \"is_closed\": " + &group_is_closed.to_string() + ", \"members\":[";
                                    for urec in urecs {
                                        resp_arr += &("{\"uid\":".to_string() + &urec.uid.to_string() +
                                            ", \"is_admin\": " + &urec.is_admin.to_string() + "},");
                                    }
                                    resp_arr += "{}]}";
                                    (defs::HTTP_STATUS_200.to_string(), resp_arr.to_string())
                                },
                                false => NOT_FOUND
                            }
                        },
                        false => LACKING_FIELDS
                    }
                },
                _ => (defs::HTTP_STATUS_403.to_string(), "{\"error\":\"Действие запрещено неавторизованным клиентам или не поддерживается\"}".to_string())
            };
        }
    } else {
        (s, c) = (defs::HTTP_STATUS_400.to_string(), "{\"error\":\"Действие не задано\"}".to_string());
    }
    return (s.to_string(), c.to_string());
}