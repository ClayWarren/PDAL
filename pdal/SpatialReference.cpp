/******************************************************************************
 * Copyright (c) 2009, Howard Butler
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of the Martin Isenburg or Iowa Department
 *       of Natural Resources nor the names of its contributors may be
 *       used to endorse or promote products derived from this software
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include <memory>

#include <pdal/Metadata.hpp>
#include <pdal/PDALUtils.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/private/SrsTransform.hpp>
#include <pdal/util/FileUtils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

// gdal
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wfloat-equal"
#include <ogr_spatialref.h>
#pragma GCC diagnostic pop

#include <cpl_conv.h>

#include <pdal/util/Utils.hpp>

namespace
{

using OGRScopedSpatialReference = std::unique_ptr<OGRSpatialReference>;

OGRScopedSpatialReference ogrCreateSrs(std::string s = "", double epoch = 0.0)
{
    OGRScopedSpatialReference r(
        new OGRSpatialReference(s.size() ? s.c_str() : nullptr));
    if (!pdal::Utils::compare_approx(epoch, 0.0f, 0.00001f))
    {
#if GDAL_VERSION_NUM >= GDAL_COMPUTE_VERSION(3, 4, 0)
        r->SetCoordinateEpoch(epoch);
#endif
    }

    return r;
}

std::string exportToWkt(OGRSpatialReference* srs,
                        const std::vector<std::string>& options = {})
{
    std::string wkt;
    if (!srs)
        return wkt;

    // Make one more pointer than option to terminate the list with a nullptr.
    std::vector<const char*> copts(options.size() + 1, nullptr);
    for (size_t i = 0; i < options.size(); ++i)
        copts[i] = options[i].c_str();

    char* buf = nullptr;
    srs->exportToWkt(&buf, copts.data());
    if (buf)
    {
        wkt = buf;
        CPLFree(buf);
    }
    return wkt;
}

} // namespace

namespace pdal
{

SpatialReference::SpatialReference(const std::string& s)
{
    set(s);
}

// NOTE that this ctor allows a string constant to be used in places
//  where a SpatialReference is extpected.
SpatialReference::SpatialReference(const char* s)
{
    set(s);
}

bool SpatialReference::empty() const
{
    return m_wkt.empty();
}

bool SpatialReference::valid() const
{
    bool v = false;
    if (pdal_srs_valid(m_wkt.c_str(), &v))
        return v;
    return false;
}

std::string SpatialReference::identifyHorizontalEPSG() const
{
    char* rust_code = nullptr;
    if (pdal_srs_identify_horizontal_epsg(m_wkt.c_str(), m_epoch, &rust_code))
    {
        std::string code(rust_code ? rust_code : "");
        pdal_string_free(rust_code);
        return code;
    }
    if (rust_code)
        pdal_string_free(rust_code);
    return "";
}

std::string SpatialReference::identifyVerticalEPSG() const
{
    char* rust_code = nullptr;
    if (pdal_srs_identify_vertical_epsg(m_wkt.c_str(), m_epoch, &rust_code))
    {
        std::string code(rust_code ? rust_code : "");
        pdal_string_free(rust_code);
        return code;
    }
    if (rust_code)
        pdal_string_free(rust_code);
    return "";
}

std::string SpatialReference::getWKT() const
{
    return m_wkt;
}

double SpatialReference::getEpoch() const
{
    return m_epoch;
}

void SpatialReference::setEpoch(const double& epoch)
{
    m_epoch = epoch;
}

std::string SpatialReference::getPROJJSON() const
{
    char* rust_json = nullptr;
    if (pdal_srs_wkt_to_projjson(m_wkt.c_str(), m_epoch, &rust_json))
    {
        std::string json(rust_json ? rust_json : "");
        pdal_string_free(rust_json);
        return json;
    }
    if (rust_json)
        pdal_string_free(rust_json);
    return std::string();
}

void SpatialReference::parse(const std::string& s, std::string::size_type& pos)
{
    set(s.substr(pos));
}

void SpatialReference::set(std::string v)
{
    m_wkt.clear();
    m_wkt2.clear();
    m_horizontalWkt.clear();
    if (v.empty())
    {
        return;
    }

    if (isWKT2(v))
    {
        m_wkt = v;
        m_wkt2 = v;
        return;
    }
    else if (isWKT1(v))
    {
        m_wkt = v;
        OGRScopedSpatialReference srs = ogrCreateSrs(m_wkt);
        if (srs)
            m_wkt2 = exportToWkt(srs.get(), {"FORMAT=WKT2_2018"});
        return;
    }

    char* rust_wkt = nullptr;
    char* rust_wkt2 = nullptr;
    double rust_epoch = 0.0;
    if (pdal_srs_user_input_to_wkt(v.c_str(), &rust_wkt, &rust_wkt2,
                                   &rust_epoch))
    {
        m_wkt = rust_wkt ? std::string(rust_wkt) : std::string();
        m_wkt2 = rust_wkt2 ? std::string(rust_wkt2) : std::string();
        m_epoch = rust_epoch;
        pdal_string_free(rust_wkt);
        pdal_string_free(rust_wkt2);
        return;
    }

    if (rust_wkt)
        pdal_string_free(rust_wkt);
    if (rust_wkt2)
        pdal_string_free(rust_wkt2);

    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error("Could not import coordinate system '" + v + "'.");
}

std::string SpatialReference::getProj4() const
{
    char* rust_proj4 = nullptr;
    if (pdal_srs_wkt_to_proj4(m_wkt.c_str(), &rust_proj4))
    {
        std::string tmp(rust_proj4 ? rust_proj4 : "");
        pdal_string_free(rust_proj4);
        return tmp;
    }
    if (rust_proj4)
        pdal_string_free(rust_proj4);
    return std::string();
}

std::string SpatialReference::getVertical() const
{
    char* rust_wkt = nullptr;
    if (pdal_srs_get_vertical_wkt(m_wkt.c_str(), &rust_wkt))
    {
        std::string out(rust_wkt ? rust_wkt : "");
        pdal_string_free(rust_wkt);
        return out;
    }
    if (rust_wkt)
        pdal_string_free(rust_wkt);
    return std::string();
}

std::string SpatialReference::getVerticalUnits() const
{
    char* rust_units = nullptr;
    if (pdal_srs_get_vertical_units(m_wkt.c_str(), &rust_units))
    {
        std::string out(rust_units ? rust_units : "");
        pdal_string_free(rust_units);
        return out;
    }
    if (rust_units)
        pdal_string_free(rust_units);
    return std::string();
}

std::string SpatialReference::getHorizontal() const
{
    if (m_horizontalWkt.empty())
    {
        char* rust_wkt = nullptr;
        if (pdal_srs_get_horizontal_wkt(m_wkt.c_str(), &rust_wkt))
        {
            if (rust_wkt)
                m_horizontalWkt = rust_wkt;
            pdal_string_free(rust_wkt);
        }
        else if (rust_wkt)
        {
            pdal_string_free(rust_wkt);
        }
    }
    return m_horizontalWkt;
}

std::string SpatialReference::getHorizontalUnits() const
{
    char* rust_units = nullptr;
    if (pdal_srs_get_horizontal_units(m_wkt.c_str(), &rust_units))
    {
        std::string out(rust_units ? rust_units : "");
        pdal_string_free(rust_units);
        return out;
    }
    if (rust_units)
        pdal_string_free(rust_units);
    return std::string();
}

bool SpatialReference::equals(const SpatialReference& input) const
{
    if (getWKT() == input.getWKT())
        return true;

    bool same = false;
    if (pdal_srs_is_same(getWKT().c_str(), input.getWKT().c_str(), m_epoch,
                         &same))
        return same;
    return false;
}

bool SpatialReference::operator==(const SpatialReference& input) const
{
    return this->equals(input);
}

bool SpatialReference::operator!=(const SpatialReference& input) const
{
    return !(this->equals(input));
}

const std::string& SpatialReference::getName() const
{
    static std::string name("pdal.spatialreference");
    return name;
}

bool SpatialReference::isGeographic() const
{
    OGRScopedSpatialReference current = ogrCreateSrs(m_wkt, m_epoch);
    if (!current)
        return false;

    bool output = current->IsGeographic();
    return output;
}

bool SpatialReference::isGeocentric() const
{
    OGRScopedSpatialReference current = ogrCreateSrs(m_wkt, m_epoch);
    if (!current)
        return false;

    bool output = current->IsGeocentric();
    return output;
}

bool SpatialReference::isProjected() const
{
    OGRScopedSpatialReference current = ogrCreateSrs(m_wkt, m_epoch);
    if (!current)
        return false;

    bool output = current->IsProjected();
    return output;
}

std::vector<int> SpatialReference::getAxisOrdering() const
{
    std::vector<int> output;
    OGRScopedSpatialReference current = ogrCreateSrs(m_wkt, m_epoch);
    if (current)
        output = current->GetDataAxisToSRSAxisMapping();
    return output;
}

int SpatialReference::calculateZone(double lon, double lat)
{
    return pdal_spatial_reference_calculate_zone(lon, lat);
}

/**
  Create a spatial reference that represents a specific UTM zone.

  \param zone  Zone number.  Must be non-zero and <= 60 and >= -60
  \return  A SpatialReference that represents the specified zone, or
    an invalid SpatialReference on error.
*/
SpatialReference SpatialReference::wgs84FromZone(int zone)
{
    char* code = pdal_spatial_reference_wgs84_code_from_zone(zone);
    std::string output(code ? code : "");
    pdal_string_free(code);
    if (output.empty())
        return SpatialReference();
    return SpatialReference(output);
}

bool SpatialReference::isWKT2(const std::string& wkt)
{
    StringList leaders{
        "GEODCRS",       "GEODETICCRS",    "GEOGCRS",     "GEOGRAPHICCRS",
        "PROJCRS",       "PROJECTEDCRS",   "VERTCRS",     "VERTICALCRS",
        "ENGCRS",        "ENGINEERINGCRS", "BOUNDCRS",    "IMAGECRS",
        "PARAMETRICCRS", "TIMECRS",        "COMPOUNDCRS", "DERIVEDPROJCRS"};

    for (const std::string& s : leaders)
        if (wkt.compare(0, s.size(), s) == 0)
            return true;
    return false;
}

bool SpatialReference::isWKT1(const std::string& wkt)
{
    // List comes from GDAL.  WKT includes FITTED_CS, but this isn't
    // included in GDAL list.  Not sure why.
    StringList leaders{"PROJCS", "GEOGCS",  "COMPD_CS",
                       "GEOCCS", "VERT_CS", "LOCAL_CS"};

    for (const std::string& s : leaders)
        if (wkt.compare(0, s.size(), s) == 0)
            return true;
    return false;
}

bool SpatialReference::isWKT(const std::string& wkt)
{
    return isWKT1(wkt) || isWKT2(wkt);
}

std::string SpatialReference::prettyWkt(const std::string& wkt)
{
    std::string outWkt;

    OGRScopedSpatialReference srs = ogrCreateSrs(wkt);
    if (!srs)
        return outWkt;

    outWkt = exportToWkt(srs.get(),
                         {"MULTILINE=YES"}); // equivalent to exportToPrettyWkt
    return outWkt;
}

std::string SpatialReference::getWKT1() const
{
    std::string wkt = getWKT();
    if (wkt.empty())
        return wkt;

    OGRScopedSpatialReference srs = ogrCreateSrs(wkt, m_epoch);
    std::string wkt1 = exportToWkt(
        srs.get(),
        {"FORMAT=WKT1_GDAL", "ALLOW_ELLIPSOIDAL_HEIGHT_AS_VERTICAL_CRS=YES"});
    if (wkt1.empty())
        throw pdal_error(
            "Couldn't convert spatial reference to WKT version 1.");
    return wkt1;
}

std::string SpatialReference::getWKT2() const
{
    return m_wkt2;
}

int SpatialReference::getUTMZone() const
{
    int32_t zone = 0;
    if (pdal_srs_get_utm_zone(m_wkt.c_str(), &zone))
        return zone;
    throw pdal_error("Could not fetch current SRS");
}

int SpatialReference::computeUTMZone(const BOX3D& cbox) const
{
    SrsTransform transform(*this, SpatialReference("EPSG:4326"));

    // We made the argument constant so copy so that we can modify.
    BOX3D box(cbox);

    transform.transform(box.minx, box.miny, box.minz);
    transform.transform(box.maxx, box.maxy, box.maxz);

    int minZone = calculateZone(box.minx, box.miny);
    int maxZone = calculateZone(box.maxx, box.maxy);

    if (minZone != maxZone)
    {
        std::ostringstream msg;
        msg << "computeUTMZone failed due to zone crossing. Zones "
               "are "
            << minZone << " and " << maxZone << ".";
        throw pdal_error(msg.str());
    }
    return minZone;
}

MetadataNode SpatialReference::toMetadata() const
{
    MetadataNode root("srs");
    root.add("horizontal", getHorizontal());
    root.add("vertical", getVertical());
    root.add("isgeographic", isGeographic());
    root.add("isgeocentric", isGeocentric());
    root.add("proj4", getProj4());
    root.add("prettywkt", prettyWkt(getHorizontal()));
    root.add("wkt", getHorizontal());
    root.addWithType("json", getPROJJSON(), "json", "PROJJSON");
    root.add("compoundwkt", getWKT());
    root.add("prettycompoundwkt", prettyWkt(m_wkt));

    MetadataNode units = root.add("units");
    units.add("vertical", getVerticalUnits());
    units.add("horizontal", getHorizontalUnits());

    return root;
}

void SpatialReference::dump() const
{
    std::cout << *this;
}

std::ostream& operator<<(std::ostream& ostr, const SpatialReference& srs)
{
    ostr << SpatialReference::prettyWkt(srs.m_wkt);
    return ostr;
}

std::istream& operator>>(std::istream& istr, SpatialReference& srs)
{
    std::ostringstream oss;
    oss << istr.rdbuf();
    srs.set(oss.str());

    return istr;
}

} // namespace pdal
