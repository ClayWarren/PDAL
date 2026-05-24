#include <nitf/BandInfo.hpp>
#include <nitf/DESegment.hpp>
#include <nitf/FileHeader.hpp>
#include <nitf/FileSecurity.hpp>
#include <nitf/IOHandle.hpp>
#include <nitf/ImageSource.hpp>
#include <nitf/ImageSubheader.hpp>
#include <nitf/LookupTable.hpp>
#include <nitf/Reader.hpp>
#include <nitf/Record.hpp>
#include <nitf/SegmentSource.hpp>
#include <nitf/TRE.hpp>
#include <nitf/Writer.hpp>

#include <algorithm>
#include <cstring>
#include <exception>
#include <string>
#include <vector>

namespace
{

void set_error(char *err, size_t err_len, const std::string& message)
{
    if (!err || err_len == 0)
        return;
    const size_t len = std::min(err_len - 1, message.size());
    std::memcpy(err, message.data(), len);
    err[len] = '\0';
}

bool has_lidar_image(nitf::Record& record)
{
    nitf::ListIterator iter = record.getImages().begin();
    const nitf::Uint32 count = record.getNumImages();
    for (nitf::Uint32 i = 0; i < count; ++i, ++iter)
    {
        nitf::ImageSegment segment = *iter;
        const std::string image_id =
            segment.getSubheader().getImageId().toString();
        if (image_id == "INTENSITY " || image_id == "ELEVATION " ||
            image_id == "None      ")
            return true;
    }
    return false;
}

bool find_lidar_data(nitf::Record& record, uint64_t *offset, uint64_t *length)
{
    nitf::ListIterator iter = record.getDataExtensions().begin();
    const nitf::Uint32 count = record.getNumDataExtensions();
    for (nitf::Uint32 i = 0; i < count; ++i, ++iter)
    {
        nitf::DESegment segment = *iter;
        nitf::DESubheader subheader = segment.getSubheader();
        const std::string id = subheader.getTypeID().toString();
        const int version = static_cast<int>(subheader.getVersion());
        if (id == "LIDARA DES               " && version == 1)
        {
            const nitf::Uint64 begin = segment.getOffset();
            const nitf::Uint64 end = segment.getEnd();
            *offset = begin;
            *length = end - begin;
            return true;
        }
    }
    return false;
}

void set_header(nitf::FileHeader& header, const char *title)
{
    header.getFileHeader().set("NITF");
    header.getComplianceLevel().set("03");
    header.getSystemType().set("BF01");
    header.getOriginStationID().set("PDAL");
    header.getFileTitle().set(title ? title : "");
    header.getClassification().set("U");
    header.getMessageCopyNum().set("00000");
    header.getMessageNumCopies().set("00000");
    header.getEncrypted().set("0");
    header.getBackgroundColor().setRawData(const_cast<char*>("000"), 3);
}

void set_data_extension(nitf::Record& record)
{
    nitf::DESegment des = record.newDataExtensionSegment();
    nitf::DESubheader subheader = des.getSubheader();
    subheader.getFilePartType().set("DE");
    subheader.getTypeID().set("LIDARA DES");
    subheader.getVersion().set("01");
    subheader.getSecurityClass().set("U");
    nitf::FileSecurity security = record.getHeader().getSecurityGroup();
    subheader.setSecurityGroup(security.clone());

    nitf::TRE user_header("LIDARA DES", "raw_data");
    user_header.setField("raw_data", "not");
    nitf::Field field = user_header.getField("raw_data");
    field.setType(nitf::Field::BINARY);
    subheader.setSubheaderFields(user_header);
}

void set_image(nitf::Record& record, const double *bounds)
{
    nitf::ImageSegment image = record.newImageSegment();
    nitf::ImageSubheader subheader = image.getSubheader();
    const double minx = bounds ? bounds[0] : 0.0;
    const double miny = bounds ? bounds[1] : 0.0;
    const double maxx = bounds ? bounds[2] : 1.0;
    const double maxy = bounds ? bounds[3] : 1.0;
    double corners[4][2] = {
        {maxy, minx},
        {maxy, maxx},
        {miny, maxx},
        {miny, minx},
    };
    subheader.setCornersFromLatLons(NRT_CORNERS_DECIMAL, corners);

    nitf::FileSecurity security = record.getHeader().getSecurityGroup();
    subheader.getImageSecurityClass().set("U");
    subheader.setSecurityGroup(security.clone());

    nitf::BandInfo info;
    nitf::LookupTable table(0, 0);
    info.init(" ", " ", "N", "   ", 0, 0, table);
    std::vector<nitf::BandInfo> bands;
    bands.push_back(info);
    subheader.setPixelInformation(
        "INT", 8, 8, "R", "NODISPLY", "VIS", 1, bands);
    subheader.setBlocking(8, 8, 8, 8, "B");
    subheader.getImageId().set("None");
}

} // namespace

extern "C" int pdal_native_nitf_lidar_segment(
    const char *input, uint64_t *offset, uint64_t *length, char *err,
    size_t err_len)
{
    try
    {
        if (!input || !offset || !length)
        {
            set_error(err, err_len, "null argument");
            return 0;
        }
        if (nitf::Reader::getNITFVersion(input) == NITF_VER_UNKNOWN)
        {
            set_error(err, err_len, "Unable to determine NITF file version");
            return 0;
        }
        nitf::IOHandle io(input);
        nitf::Reader reader;
        nitf::Record record = reader.read(io);
        if (!has_lidar_image(record))
        {
            set_error(err, err_len,
                      "Unable to find lidar-compatible image segment in NITF file");
            return 0;
        }
        if (!find_lidar_data(record, offset, length))
        {
            set_error(err, err_len,
                      "Unable to find LIDARA data extension segment in NITF file");
            return 0;
        }
        return 1;
    }
    catch (const std::exception& ex)
    {
        set_error(err, err_len, ex.what());
        return 0;
    }
}

extern "C" int pdal_native_nitf_wrap(
    const char *input, const char *output, const char *title,
    const double *bounds, char *err, size_t err_len)
{
    try
    {
        if (!input || !output)
        {
            set_error(err, err_len, "null argument");
            return 0;
        }
        nitf::Record record(NITF_VER_21);
        nitf::FileHeader header = record.getHeader();
        set_header(header, title);
        set_data_extension(record);
        set_image(record, bounds);

        nitf::Writer writer;
        nitf::IOHandle output_io(output, NITF_ACCESS_WRITEONLY, NITF_CREATE);
        writer.prepare(output_io, record);

        nitf::IOHandle input_io(input);
        nitf::SegmentFileSource source(input_io, 0, 0);
        nitf::SegmentWriter segment_writer = writer.newDEWriter(0);
        segment_writer.attachSource(source);

        std::string zeros(64, '0');
        nitf::MemorySource band(
            const_cast<char*>(zeros.c_str()), zeros.size(), 0, 1, 0);
        nitf::ImageSource image_source;
        image_source.addBand(band);
        nitf::ImageWriter image_writer = writer.newImageWriter(0);
        image_writer.attachSource(image_source);

        writer.write();
        output_io.close();
        return 1;
    }
    catch (const std::exception& ex)
    {
        set_error(err, err_len, ex.what());
        return 0;
    }
}
