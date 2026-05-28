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

#include <pdal/Metadata.hpp>
#include <pdal/PDALUtils.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/private/SrsTransform.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <pdal/util/Utils.hpp>

namespace
{

std::string takePdalString(char* ptr)
{
    std::string output(ptr ? ptr : "");
    pdal_string_free(ptr);
    return output;
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
        char* rust_wkt2 = nullptr;
        if (pdal_srs_wkt_to_wkt2(m_wkt.c_str(), m_epoch, &rust_wkt2))
            m_wkt2 = takePdalString(rust_wkt2);
        else if (rust_wkt2)
            pdal_string_free(rust_wkt2);
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
    bool output = false;
    if (pdal_srs_is_geographic(m_wkt.c_str(), m_epoch, &output))
        return output;
    return false;
}

bool SpatialReference::isGeocentric() const
{
    bool output = false;
    if (pdal_srs_is_geocentric(m_wkt.c_str(), m_epoch, &output))
        return output;
    return false;
}

bool SpatialReference::isProjected() const
{
    bool output = false;
    if (pdal_srs_is_projected(m_wkt.c_str(), m_epoch, &output))
        return output;
    return false;
}

std::vector<int> SpatialReference::getAxisOrdering() const
{
    std::vector<int> output;
    uint64_t len = 0;
    int32_t* ordering = pdal_srs_axis_ordering(m_wkt.c_str(), m_epoch, &len);
    if (ordering)
    {
        output.assign(ordering, ordering + len);
        pdal_i32_array_free(ordering, len);
    }
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
    char* rust_wkt = nullptr;
    if (pdal_srs_pretty_wkt(wkt.c_str(), &rust_wkt))
        return takePdalString(rust_wkt);
    if (rust_wkt)
        pdal_string_free(rust_wkt);
    return std::string();
}

std::string SpatialReference::getWKT1() const
{
    std::string wkt = getWKT();
    if (wkt.empty())
        return wkt;

    char* rust_wkt = nullptr;
    std::string wkt1;
    if (pdal_srs_wkt_to_wkt1(wkt.c_str(), m_epoch, &rust_wkt))
        wkt1 = takePdalString(rust_wkt);
    else if (rust_wkt)
        pdal_string_free(rust_wkt);
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
