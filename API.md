# Api documentation

## Prelude

### Glossary

#### `nid` = `uuid` = nanoid (https://github.com/ai/nanoid)
Term `uuid` is being deprecated in favor of `nid`. If anywhere is mentioned `uuid`, it is meant to be `nid`.

### Versioning

**Current api version is `0`**

All api endpoints are versioned (except '/').
To access versioned endpoint, prepend path with `v<APIVERSION>/`

Example assuming `APIVERSION` equals `0`:
```
hxxp://target.instance/v0/
```

### Authorization

Currently, there's only one way of authorization - token-based.
For enhanced security, you can limit token to concrete IP address.
To access protected endpoint, just add `Authorization` header.
Example:
```
Authorization: Machine abcdefghijklmopqrstuvxyz
```
All calls to API are rate-limited.

### Response codes

todo

### Response schema

Each response from API keeps same schema:

- items index (cursorable)

```json
200 OK

{
    "count": 1,         // items count in items array,
    "cursor": <cursor>,   // cursor id to the next item | null if no more
    "items": [          // items list
        ...
    ]
}
```

- item (non-cursorable)

```json
200 OK

{
    "count": 1,         // items count in items array,
    "items": [          // items list
        ...
    ]
}
```

- item create (indexed)

```json
201 Created

{
    "nid": <nid>  // nanoid of newly created item
}
```

- item create (not indexed)

```json
201 Created
```

- item update

```json
200 OK
```

- error

```json
4XX Error

{
    "error": "short message"
}
```

API field can be one of the following type:
- string `"..."`
- integer `123`
- float `1.23`
- boolean represented as integers `0`/`1`
- timestamp represented as integer GMT `1655918138` -> `22/06/2022 17:15:38 GMT`

## Endpoints

#### _ANY_ `/`
##### Availability
**v0**

##### Description
This endpoint always responds with 403 Forbidden status. 

##### Response format
```json
{"error": "forbidden"}
```

##### Available response codes
- _403 Forbidden_