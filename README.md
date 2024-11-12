## Dynamic MVT Tiling

Precomputed mapbox vector tiles (MVT) come with size limitations or can impacted performance when trying to load too
many points or complex polygons. In addition, it becomes difficult to make your geospatial data highly filterable when
points have been dropped to meet tile size limitations. The goal of this project is to solve that by dynamically
querying the data and serving up the tiles at varying levels of detail depending on zoom level.

#### Requirements

* postgres (tested using postgres@14)
  * [extensions](https://postgis.net/documentation/getting_started/install_windows/enabling_postgis/)
    * postgis
    * h3 / h3_postgis (used to reduce points at higher zoom levels)

#### Optional

* redis-server

#### Environment Variables

Optionally can use `.env` file

```
# Required
DATABASE_URL=[postgres scheme uri i.e. postgres://user:password@localhost/db]

# Optional
CACHE_URL=[redis cache url]
ALLOWED_ORIGINS=[list of allowed origins space separated (must be surrounded by quotes if using .env file)]
DISABLE_GZIP=[disable gzip true/false]
```

#### Getting Startup

```
cargo build --release
./rs-dynamic-mvt
```

#### Usage

<details>
 <summary><code>GET</code> <code><b>/mvt/{x}/{y}/{z}</b></code> </summary>

##### Parameters

> | name |  type     | data type             | description          |
> |------|-----------|-----------------------|----------------------|
> | x    |  required | integer               | cartesian coordinate |
> | y    |  required | integer | cartesian coordinate |
> | z    |  required | integer | zoom level           |

##### Query Parameters

> | name   | type     | data type | description                                                                       |
> |--------|----------|-----------|-----------------------------------------------------------------------------------|
> | query  | required | string    | SQL query for geospatial data                                                     |
> | geoCol | required | string   | Name of geospatial column (must be included in the final select of the SQL query) |
> | srid   | optional | integer   | SRID for the geospatial column (default: 4326)                                    |

##### Responses

> | http code | content-type                      | response              |
> |-----------|-----------------------------------|-----------------------|
> | `200`     | `application/x-protobuf`        | `MVT protobuf binary` |

##### Usage Example

DeckGL Layer:

```
 const mvtLayer = new MVTLayer({
            id: 'mvtLayer',
            data: [
                'http://localhost:8095/mvt/{x}/{y}/{z}?query=SELECT id, name, location FROM my_geospatial_data&geoCol=location',
            ],
            minZoom: 0,
            maxZoom: 24,
            getFillColor: [255, 0, 0],
            getLineWidth: 1,
            pointRadiusUnits: 'pixels',
            getPointRadius: 5,
            stroked: true,
            getLineColor: [0, 0, 255],
            pickable: true
        }
    )
```

</details>

