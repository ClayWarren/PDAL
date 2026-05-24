#include <except/Throwable.h>
#include <nitf/BandInfo.hpp>
#include <nitf/DESegment.hpp>
#include <nitf/Defines.h>
#include <nitf/Extensions.hpp>
#include <nitf/FileHeader.hpp>
#include <nitf/PluginRegistry.hpp>
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
#include <cctype>
#include <cmath>
#include <cstring>
#include <exception>
#include <mutex>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

// Subset of TRE plugins needed by the NITF writer's default AIMIDB and ACFTB
// records. Loaded once on first use through register_required_tres().
NITF_TRE_STATIC_HANDLER_REF(ACFTB)
NITF_TRE_STATIC_HANDLER_REF(AIMIDB)

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

void register_required_tres()
{
    static std::once_flag flag;
    std::call_once(flag, []() {
        nitf::PluginRegistry::registerTREHandler(ACFTB_init, ACFTB_handler);
        nitf::PluginRegistry::registerTREHandler(AIMIDB_init, AIMIDB_handler);
    });
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

std::string trim_trailing(const std::string& s)
{
    size_t end = s.find_last_not_of(" \t\r\n");
    if (end == std::string::npos)
        return std::string();
    return s.substr(0, end + 1);
}

using MetaCb = int (*)(const char *key, const char *value, void *userdata);

void emit_field(const std::string& parent, const std::string& tag,
                nitf::Field field, MetaCb cb, void *userdata, bool *stop)
{
    if (*stop)
        return;
    std::string value;
    auto type = field.getType();
    if (type == (nitf::Field::FieldType)NITF_BCS_A
        || type == (nitf::Field::FieldType)NITF_BCS_N)
    {
        value = field.toString();
    }
    else if (type == (nitf::Field::FieldType)NITF_BINARY)
    {
        if (tag == "FBKGC")
        {
            std::string raw = field.toString();
            std::ostringstream oss;
            for (size_t i = 0; i < raw.size() && i < 3; ++i)
            {
                if (i > 0)
                    oss << ", ";
                oss << static_cast<unsigned int>(static_cast<unsigned char>(raw[i]));
            }
            value = oss.str();
        }
        else
        {
            value = "(binary)";
        }
    }
    std::string trimmed = trim_trailing(value);
    const std::string key = parent + "." + tag;
    if (cb(key.c_str(), trimmed.c_str(), userdata) != 0)
        *stop = true;
}

void emit_int(const std::string& parent, const std::string& tag, int value,
              MetaCb cb, void *userdata, bool *stop)
{
    if (*stop)
        return;
    const std::string key = parent + "." + tag;
    const std::string text = std::to_string(value);
    if (cb(key.c_str(), text.c_str(), userdata) != 0)
        *stop = true;
}

void emit_security(const std::string& parent, const std::string& prefix,
                   nitf::FileSecurity security, MetaCb cb, void *userdata,
                   bool *stop)
{
    emit_field(parent, prefix + "SCLSY", security.getClassificationSystem(), cb, userdata, stop);
    emit_field(parent, prefix + "SCODE", security.getCodewords(), cb, userdata, stop);
    emit_field(parent, prefix + "SCTLH", security.getControlAndHandling(), cb, userdata, stop);
    emit_field(parent, prefix + "SREL", security.getReleasingInstructions(), cb, userdata, stop);
    emit_field(parent, prefix + "SDCTP", security.getDeclassificationType(), cb, userdata, stop);
    emit_field(parent, prefix + "SDCDT", security.getDeclassificationDate(), cb, userdata, stop);
    emit_field(parent, prefix + "SDCXM", security.getDeclassificationExemption(), cb, userdata, stop);
    emit_field(parent, prefix + "SDG", security.getDowngrade(), cb, userdata, stop);
    emit_field(parent, prefix + "SDGDT", security.getDowngradeDateTime(), cb, userdata, stop);
    emit_field(parent, prefix + "SCLTX", security.getClassificationText(), cb, userdata, stop);
    emit_field(parent, prefix + "SCATP", security.getClassificationAuthorityType(), cb, userdata, stop);
    emit_field(parent, prefix + "SCAUT", security.getClassificationAuthority(), cb, userdata, stop);
    emit_field(parent, prefix + "SCRSN", security.getClassificationReason(), cb, userdata, stop);
    emit_field(parent, prefix + "SSRDT", security.getSecuritySourceDate(), cb, userdata, stop);
    emit_field(parent, prefix + "SCTLN", security.getSecurityControlNumber(), cb, userdata, stop);
}

void emit_tre(const std::string& parent, nitf::TRE tre, MetaCb cb,
              void *userdata, bool *stop)
{
    const std::string tag = parent + "." + tre.getTag();
    auto iter = tre.begin();
    while (iter != tre.end() && !*stop)
    {
        try
        {
            nitf::Pair pair = *iter;
            const char *key = pair.first();
            if (key && std::strcmp(key, "raw_data") != 0)
            {
                nitf::Field field = tre.getField(key);
                emit_field(tag, key, field, cb, userdata, stop);
            }
        }
        catch (const except::NullPointerReference&)
        {
        }
        ++iter;
    }
}

void emit_extensions(const std::string& parent, nitf::Extensions ext,
                     MetaCb cb, void *userdata, bool *stop)
{
    auto iter = ext.begin();
    while (iter != ext.end() && !*stop)
    {
        nitf::TRE tre = *iter;
        emit_tre(parent, tre, cb, userdata, stop);
        ++iter;
    }
}

void emit_file_header(nitf::FileHeader header, MetaCb cb, void *userdata,
                      bool *stop)
{
    emit_field("FH", "FHDR", header.getFileHeader(), cb, userdata, stop);
    emit_field("FH", "FVER", header.getFileVersion(), cb, userdata, stop);
    emit_field("FH", "CLEVEL", header.getComplianceLevel(), cb, userdata, stop);
    emit_field("FH", "STYPE", header.getSystemType(), cb, userdata, stop);
    emit_field("FH", "OSTAID", header.getOriginStationID(), cb, userdata, stop);
    emit_field("FH", "FDT", header.getFileDateTime(), cb, userdata, stop);
    emit_field("FH", "FTITLE", header.getFileTitle(), cb, userdata, stop);
    emit_field("FH", "FSCLAS", header.getClassification(), cb, userdata, stop);
    emit_field("FH", "FSCOP", header.getMessageCopyNum(), cb, userdata, stop);
    emit_field("FH", "FSCPYS", header.getMessageNumCopies(), cb, userdata, stop);
    emit_field("FH", "ENCRYP", header.getEncrypted(), cb, userdata, stop);
    emit_field("FH", "FBKGC", header.getBackgroundColor(), cb, userdata, stop);
    emit_field("FH", "ONAME", header.getOriginatorName(), cb, userdata, stop);
    emit_field("FH", "OPHONE", header.getOriginatorPhone(), cb, userdata, stop);
    emit_field("FH", "FL", header.getFileLength(), cb, userdata, stop);
    emit_field("FH", "HL", header.getHeaderLength(), cb, userdata, stop);
    emit_field("FH", "NUMI", header.getNumImages(), cb, userdata, stop);
    emit_field("FH", "NUMS", header.getNumGraphics(), cb, userdata, stop);
    emit_field("FH", "NUMT", header.getNumTexts(), cb, userdata, stop);
    emit_field("FH", "NUMDES", header.getNumDataExtensions(), cb, userdata, stop);
    emit_field("FH", "NUMRES", header.getNumReservedExtensions(), cb, userdata, stop);
}

void emit_band(const std::string& key, nitf::BandInfo band, MetaCb cb,
               void *userdata, bool *stop)
{
    emit_field(key, "IREPBAND", band.getRepresentation(), cb, userdata, stop);
    emit_field(key, "ISUBCAT", band.getSubcategory(), cb, userdata, stop);
    emit_field(key, "IFC", band.getImageFilterCondition(), cb, userdata, stop);
    emit_field(key, "IMFLT", band.getImageFilterCode(), cb, userdata, stop);
    emit_field(key, "NLUTS", band.getNumLUTs(), cb, userdata, stop);
    emit_field(key, "NELUT", band.getBandEntriesPerLUT(), cb, userdata, stop);
    nitf::LookupTable lut = band.getLookupTable();
    emit_int(key, "num_lookup_tables", lut.getTables(), cb, userdata, stop);
    emit_int(key, "num_lookup_entries", lut.getEntries(), cb, userdata, stop);
}

void emit_image_subheader(const std::string& key, nitf::ImageSubheader sub,
                          MetaCb cb, void *userdata, bool *stop)
{
    emit_field(key, "IID1", sub.getImageId(), cb, userdata, stop);
    emit_field(key, "IDATIM", sub.getImageDateAndTime(), cb, userdata, stop);
    emit_field(key, "TGTID", sub.getTargetId(), cb, userdata, stop);
    emit_field(key, "IID2", sub.getImageTitle(), cb, userdata, stop);
    emit_field(key, "ISCLAS", sub.getImageSecurityClass(), cb, userdata, stop);
    nitf::FileSecurity security = sub.getSecurityGroup();
    emit_security(key, "I", security, cb, userdata, stop);
    emit_field(key, "ENCRYP", sub.getEncrypted(), cb, userdata, stop);
    emit_field(key, "ISORCE", sub.getImageSource(), cb, userdata, stop);
    emit_field(key, "NROWS", sub.getNumRows(), cb, userdata, stop);
    emit_field(key, "NCOLS", sub.getNumCols(), cb, userdata, stop);
    emit_field(key, "PVTYPE", sub.getPixelValueType(), cb, userdata, stop);
    emit_field(key, "IREP", sub.getImageRepresentation(), cb, userdata, stop);
    emit_field(key, "ICAT", sub.getImageCategory(), cb, userdata, stop);
    emit_field(key, "ABPP", sub.getActualBitsPerPixel(), cb, userdata, stop);
    emit_field(key, "PJUST", sub.getPixelJustification(), cb, userdata, stop);
    emit_field(key, "ICORDS", sub.getImageCoordinateSystem(), cb, userdata, stop);
    emit_field(key, "IGEOLO", sub.getCornerCoordinates(), cb, userdata, stop);
    emit_field(key, "NICOM", sub.getNumImageComments(), cb, userdata, stop);
    nitf::List comments = sub.getImageComments();
    int comment_index = 0;
    for (auto cit = comments.begin(); cit != comments.end() && !*stop; ++cit)
    {
        nitf::Field field = *cit;
        emit_field(key, "ICOM:" + std::to_string(comment_index), field, cb, userdata, stop);
        ++comment_index;
    }
    emit_field(key, "IC", sub.getImageCompression(), cb, userdata, stop);
    emit_field(key, "COMRAT", sub.getCompressionRate(), cb, userdata, stop);
    emit_field(key, "NBANDS", sub.getNumImageBands(), cb, userdata, stop);
    emit_field(key, "XBANDS", sub.getNumMultispectralImageBands(), cb, userdata, stop);
    const int nbands = static_cast<int>(sub.getNumImageBands());
    for (int i = 0; i < nbands && !*stop; ++i)
    {
        nitf::BandInfo band = sub.getBandInfo(i);
        const std::string subkey = key + ".BAND:" + std::to_string(i);
        emit_band(subkey, band, cb, userdata, stop);
    }
    emit_field(key, "ISYNC", sub.getImageSyncCode(), cb, userdata, stop);
    emit_field(key, "IMODE", sub.getImageMode(), cb, userdata, stop);
    emit_field(key, "NBPR", sub.getNumBlocksPerRow(), cb, userdata, stop);
    emit_field(key, "NBPC", sub.getNumBlocksPerCol(), cb, userdata, stop);
    emit_field(key, "NPPBH", sub.getNumPixelsPerHorizBlock(), cb, userdata, stop);
    emit_field(key, "NPPVB", sub.getNumPixelsPerVertBlock(), cb, userdata, stop);
    emit_field(key, "NBPP", sub.getNumBitsPerPixel(), cb, userdata, stop);
    emit_field(key, "IDLVL", sub.getImageDisplayLevel(), cb, userdata, stop);
    emit_field(key, "IALVL", sub.getImageAttachmentLevel(), cb, userdata, stop);
    emit_field(key, "ILOC", sub.getImageLocation(), cb, userdata, stop);
    emit_field(key, "IMAG", sub.getImageMagnification(), cb, userdata, stop);
}

void emit_de_subheader(const std::string& key, nitf::DESubheader sub,
                       MetaCb cb, void *userdata, bool *stop)
{
    emit_field(key, "DESID", sub.getTypeID(), cb, userdata, stop);
    emit_field(key, "DESVER", sub.getVersion(), cb, userdata, stop);
    emit_field(key, "DECLAS", sub.getSecurityClass(), cb, userdata, stop);
    nitf::FileSecurity security = sub.getSecurityGroup();
    emit_security(key, "DE", security, cb, userdata, stop);
}

void apply_aimidb_acftb(nitf::TRE& tre, const char *const *fields)
{
    if (!fields)
        return;
    for (const char *const *it = fields; *it != nullptr; ++it)
    {
        std::string entry(*it);
        size_t colon = entry.find(':');
        if (colon == std::string::npos)
            throw std::runtime_error("Invalid AIMIDB/ACFTB entry '"
                                     + entry + "'. Expected name:value.");
        std::string name = entry.substr(0, colon);
        std::string value = entry.substr(colon + 1);
        while (!name.empty() && std::isspace(static_cast<unsigned char>(name.back())))
            name.pop_back();
        while (!name.empty() && std::isspace(static_cast<unsigned char>(name.front())))
            name.erase(name.begin());
        while (!value.empty() && std::isspace(static_cast<unsigned char>(value.back())))
            value.pop_back();
        while (!value.empty() && std::isspace(static_cast<unsigned char>(value.front())))
            value.erase(value.begin());
        tre.setField(name, value);
    }
}

} // namespace

struct pdal_native_nitf_write_options
{
    const char *file_title;
    const char *complexity_level;
    const char *system_type;
    const char *origin_station_id;
    const char *file_class;
    const char *origin_name;
    const char *origin_phone;
    const char *fsclsy;
    const char *fsctlh;
    const char *fscltx;
    const char *image_security_class;
    const char *image_date_time;
    const char *image_id2;
    const char *const *aimidb;
    const char *const *acftb;
    double minx;
    double miny;
    double maxx;
    double maxy;
};

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
    catch (const except::Throwable& ex)
    {
        set_error(err, err_len, ex.getMessage());
        return 0;
    }
    catch (const std::exception& ex)
    {
        set_error(err, err_len, ex.what());
        return 0;
    }
    catch (...)
    {
        set_error(err, err_len, "unknown NITF error");
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
    catch (const except::Throwable& ex)
    {
        set_error(err, err_len, ex.getMessage());
        return 0;
    }
    catch (const std::exception& ex)
    {
        set_error(err, err_len, ex.what());
        return 0;
    }
    catch (...)
    {
        set_error(err, err_len, "unknown NITF error");
        return 0;
    }
}

extern "C" int pdal_native_nitf_read_metadata(
    const char *input,
    int (*cb)(const char *key, const char *value, void *userdata),
    void *userdata,
    char *err, size_t err_len)
{
    try
    {
        if (!input || !cb)
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
        bool stop = false;

        nitf::FileHeader header = record.getHeader();
        emit_file_header(header, cb, userdata, &stop);
        nitf::FileSecurity security = header.getSecurityGroup();
        emit_security("FH", "F", security, cb, userdata, &stop);
        nitf::Extensions extensions = header.getExtendedSection();
        emit_extensions("FH", extensions, cb, userdata, &stop);

        nitf::List images = record.getImages();
        const nitf::Uint32 num_images = record.getNumImages();
        auto image_iter = images.begin();
        for (nitf::Uint32 i = 0; i < num_images && !stop; ++i, ++image_iter)
        {
            const std::string key = "IM:" + std::to_string(i);
            nitf::ImageSegment segment = *image_iter;
            nitf::ImageSubheader subheader = segment.getSubheader();
            emit_image_subheader(key, subheader, cb, userdata, &stop);
            nitf::Extensions image_ext = subheader.getExtendedSection();
            emit_extensions(key, image_ext, cb, userdata, &stop);
            nitf::Extensions user_ext = subheader.getUserDefinedSection();
            emit_extensions(key, user_ext, cb, userdata, &stop);
        }

        nitf::List des = record.getDataExtensions();
        const nitf::Uint32 num_des = record.getNumDataExtensions();
        auto des_iter = des.begin();
        for (nitf::Uint32 i = 0; i < num_des && !stop; ++i, ++des_iter)
        {
            const std::string key = "DE:" + std::to_string(i);
            nitf::DESegment segment = *des_iter;
            nitf::DESubheader subheader = segment.getSubheader();
            emit_de_subheader(key, subheader, cb, userdata, &stop);
            nitf::Extensions user_ext = subheader.getUserDefinedSection();
            emit_extensions(key, user_ext, cb, userdata, &stop);
        }

        return 1;
    }
    catch (const except::Throwable& ex)
    {
        set_error(err, err_len, ex.getMessage());
        return 0;
    }
    catch (const std::exception& ex)
    {
        set_error(err, err_len, ex.what());
        return 0;
    }
    catch (...)
    {
        set_error(err, err_len, "unknown NITF error");
        return 0;
    }
}

extern "C" int pdal_native_nitf_write(
    const char *input, const char *output,
    const struct pdal_native_nitf_write_options *opts,
    char *err, size_t err_len)
{
    try
    {
        if (!input || !output || !opts)
        {
            set_error(err, err_len, "null argument");
            return 0;
        }

        register_required_tres();

        nitf::Record record(NITF_VER_21);
        nitf::FileHeader header = record.getHeader();

        const char *complexity = opts->complexity_level && *opts->complexity_level
                                     ? opts->complexity_level
                                     : "03";
        const char *system_type = opts->system_type && *opts->system_type
                                      ? opts->system_type
                                      : "BF01";
        const char *ostaid = opts->origin_station_id && *opts->origin_station_id
                                 ? opts->origin_station_id
                                 : "PDAL";
        const char *file_class = opts->file_class && *opts->file_class
                                     ? opts->file_class
                                     : "U";
        const char *isclas = opts->image_security_class
                                         && *opts->image_security_class
                                     ? opts->image_security_class
                                     : "U";
        std::string file_title = opts->file_title ? opts->file_title : "";

        header.getFileHeader().set("NITF");
        header.getComplianceLevel().set(complexity);
        header.getSystemType().set(system_type);
        header.getOriginStationID().set(ostaid);
        if (file_title.size() > header.getFileTitle().getLength())
        {
            std::string msg = "Can't write file.  FTITLE field (usually "
                              "filename) can't be longer than "
                + std::to_string(header.getFileTitle().getLength())
                + ".  Use 'ftitle' option to set appropriately sized FTITLE.";
            set_error(err, err_len, msg);
            return 0;
        }
        header.getFileTitle().set(file_title);
        header.getClassification().set(file_class);
        header.getMessageCopyNum().set("00000");
        header.getMessageNumCopies().set("00000");
        header.getEncrypted().set("0");
        header.getBackgroundColor().setRawData(const_cast<char*>("000"), 3);
        if (opts->origin_name)
            header.getOriginatorName().set(opts->origin_name);
        if (opts->origin_phone)
            header.getOriginatorPhone().set(opts->origin_phone);
        if (opts->fsclsy)
            header.getSecurityGroup().getClassificationSystem().set(opts->fsclsy);
        if (opts->fsctlh)
            header.getSecurityGroup().getControlAndHandling().set(opts->fsctlh);
        if (opts->fscltx)
            header.getSecurityGroup().getClassificationText().set(opts->fscltx);

        nitf::DESegment des = record.newDataExtensionSegment();
        des.getSubheader().getFilePartType().set("DE");
        des.getSubheader().getTypeID().set("LIDARA DES");
        des.getSubheader().getVersion().set("01");
        des.getSubheader().getSecurityClass().set(file_class);
        nitf::FileSecurity record_security = record.getHeader().getSecurityGroup();
        des.getSubheader().setSecurityGroup(record_security.clone());

        nitf::TRE user_header("LIDARA DES", "raw_data");
        user_header.setField("raw_data", "not");
        nitf::Field field = user_header.getField("raw_data");
        field.setType(nitf::Field::BINARY);
        des.getSubheader().setSubheaderFields(user_header);

        nitf::ImageSegment image = record.newImageSegment();
        nitf::ImageSubheader sub = image.getSubheader();

        // NITF wants corners as decimal degrees; quantize to 3dp.
        double minx = std::floor(opts->minx * 1000.0) / 1000.0;
        double miny = std::floor(opts->miny * 1000.0) / 1000.0;
        double maxx = std::ceil(opts->maxx * 1000.0) / 1000.0;
        double maxy = std::ceil(opts->maxy * 1000.0) / 1000.0;
        double corners[4][2] = {
            {maxy, minx},
            {maxy, maxx},
            {miny, maxx},
            {miny, minx},
        };
        sub.setCornersFromLatLons(NRT_CORNERS_DECIMAL, corners);
        sub.getImageSecurityClass().set(isclas);
        sub.setSecurityGroup(record_security.clone());

        std::string image_date = opts->image_date_time ? opts->image_date_time : "";
        if (!image_date.empty())
            sub.getImageDateAndTime().set(image_date);

        nitf::BandInfo info;
        nitf::LookupTable lt(0, 0);
        info.init(" ", " ", "N", "   ", 0, 0, lt);
        std::vector<nitf::BandInfo> bands;
        bands.push_back(info);
        sub.setPixelInformation("INT", 8, 8, "R", "NODISPLY", "VIS", 1, bands);
        sub.setBlocking(8, 8, 8, 8, "B");
        sub.getImageId().set("None");
        if (opts->image_id2)
            sub.getImageTitle().set(opts->image_id2);

        nitf::TRE aimidb_tre("AIMIDB");
        if (!image_date.empty())
            aimidb_tre.setField("ACQUISITION_DATE", image_date);
        aimidb_tre.setField("MISSION_NO", "UNKN");
        aimidb_tre.setField("MISSION_IDENTIFICATION", "NOT AVAIL.");
        aimidb_tre.setField("FLIGHT_NO", "00");
        aimidb_tre.setField("CURRENT_SEGMENT", "AA");
        aimidb_tre.setField("START_TILE_COLUMN", "001");
        aimidb_tre.setField("START_TILE_ROW", "00001");
        aimidb_tre.setField("END_SEGMENT", "00");
        aimidb_tre.setField("END_TILE_COLUMN", "001");
        aimidb_tre.setField("END_TILE_ROW", "00001");
        apply_aimidb_acftb(aimidb_tre, opts->aimidb);
        sub.getExtendedSection().appendTRE(aimidb_tre);

        if (image_date.empty())
        {
            std::string acq = aimidb_tre.getField("ACQUISITION_DATE").toString();
            if (!acq.empty())
            {
                image_date = acq;
                sub.getImageDateAndTime().set(image_date);
            }
        }

        nitf::TRE acftb_tre("ACFTB");
        acftb_tre.setField("AC_MSN_ID", "NOT AVAILABLE");
        acftb_tre.setField("SCENE_SOURCE", " ");
        if (image_date.size() > 7)
            acftb_tre.setField("PDATE", image_date.substr(0, 8));
        acftb_tre.setField("MPLAN", "999");
        acftb_tre.setField("LOC_ACCY", "000.00");
        acftb_tre.setField("ROW_SPACING", "0000000");
        acftb_tre.setField("ROW_SPACING_UNITS", "u");
        acftb_tre.setField("COL_SPACING", "0000000");
        acftb_tre.setField("COL_SPACING_UNITS", "u");
        acftb_tre.setField("FOCAL_LENGTH", "999.99");
        apply_aimidb_acftb(acftb_tre, opts->acftb);
        sub.getExtendedSection().appendTRE(acftb_tre);

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
    catch (const except::Throwable& ex)
    {
        set_error(err, err_len, ex.getMessage());
        return 0;
    }
    catch (const std::exception& ex)
    {
        set_error(err, err_len, ex.what());
        return 0;
    }
    catch (...)
    {
        set_error(err, err_len, "unknown NITF error");
        return 0;
    }
}
