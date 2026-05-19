#include "OGRSpec.hpp"
#include <pdal/private/gdal/GDALUtils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <nlohmann/json.hpp>

namespace pdal
{

OGRSpec::OGRSpec() {}

OGRSpec::OGRSpec(const std::string& ogrJsonStr)
{
    validateInput(ogrJsonStr);
    initialize();
}

OGRSpec::OGRSpec(const NL::json& ogrJson)
{
    validateInput(ogrJson);
    initialize();
}

void OGRSpec::update(const std::string& ogrJsonStr)
{
    m_geom.clear();
    validateInput(ogrJsonStr);
    initialize();
}

void OGRSpec::update(const NL::json& ogrJson)
{
    m_geom.clear();
    validateInput(ogrJson);
    initialize();
}

void OGRSpec::validateInput(const std::string& ogrJsonStr)
{
    try
    {
        m_json = NL::json::parse(ogrJsonStr);
    }
    catch (NL::json::parse_error& e)
    {
        std::string s(e.what());
        auto pos = s.find(']');
        if (pos != std::string::npos)
            s = s.substr(pos + 1);
        std::stringstream msg;

        msg << "Failed to parse OGR JSON with error: " << s;
        throw error(msg.str());
    }
    parse();
}

void OGRSpec::validateInput(const NL::json& ogrJson)
{
    m_json = ogrJson;
    parse();
}

void OGRSpec::parse()
{
    char* parsed = pdal_ogr_spec_parse_json(m_json.dump().c_str());
    if (!parsed)
        throw error("Failed to parse OGR JSON.");

    NL::json result = NL::json::parse(parsed);
    pdal_string_free(parsed);

    if (!result.value("ok", false))
        throw error(result.value("error", "Failed to parse OGR JSON."));

    m_opts = {};
    assignJSON(result.at("datasource"), m_opts.datasource);
    assignJSON(result.at("drivers"), m_opts.drivers);
    assignJSON(result.at("openoptions"), m_opts.openOpts);
    assignJSON(result.at("layer"), m_opts.layer);
    assignJSON(result.at("sql"), m_opts.sql);
    assignJSON(result.at("dialect"), m_opts.dialect);
    assignJSON(result.at("geometry"), m_opts.geometry);
}

void OGRSpec::initialize()
{
    m_geom = gdal::getPolygons(m_opts);
}

std::ostream& operator<<(std::ostream& out, const OGRSpec& ogr)
{
    out << ogr.m_json;
    return out;
}

} // namespace pdal
