#include <cstdint>
#include <cstdlib>
#include <cstring>

#include <geo_normalize.h>
#include <geo_simpletags.h>
#include <geo_tiffp.h>

extern "C"
{
    char* GTIFGetOGISDefn(GTIF*, GTIFDefn*);
    void VSIFree(void* data);
}

namespace
{

struct GeotiffCtx
{
    GeotiffCtx()
    {
        tiff = ST_Create();
        GTIFSetSimpleTagsMethods(&methods);
    }

    ~GeotiffCtx()
    {
        if (gtiff)
            GTIFFree(gtiff);
        ST_Destroy(tiff);
    }

    ST_TIFF* tiff = nullptr;
    GTIF* gtiff = nullptr;
    TIFFMethod methods;
};

std::uint16_t le16(const std::uint8_t* data)
{
    return static_cast<std::uint16_t>(data[0]) |
           static_cast<std::uint16_t>(data[1] << 8);
}

char* duplicate(const char* value)
{
    if (!value)
        return nullptr;
    const std::size_t len = std::strlen(value);
    char* out = static_cast<char*>(std::malloc(len + 1));
    if (!out)
        return nullptr;
    std::memcpy(out, value, len + 1);
    return out;
}

} // namespace

extern "C"
{

char* pdal_native_geotiff_wkt(const std::uint8_t* directory,
                              std::size_t directory_len,
                              const std::uint8_t* doubles,
                              std::size_t doubles_len,
                              const std::uint8_t* ascii, std::size_t ascii_len)
{
    if (!directory || directory_len < 8)
        return nullptr;

    GeotiffCtx ctx;
    const std::uint16_t num_keys = le16(directory + 6);
    const std::size_t short_count = (static_cast<std::size_t>(num_keys) + 1) * 4;
    if (directory_len < short_count * sizeof(std::uint16_t))
        return nullptr;

    ST_SetKey(ctx.tiff, 34735, static_cast<int>(short_count), STT_SHORT,
              const_cast<std::uint8_t*>(directory));

    if (doubles && doubles_len >= sizeof(double))
        ST_SetKey(ctx.tiff, 34736, static_cast<int>(doubles_len / sizeof(double)),
                  STT_DOUBLE, const_cast<std::uint8_t*>(doubles));

    if (ascii && ascii_len)
        ST_SetKey(ctx.tiff, 34737, static_cast<int>(ascii_len), STT_ASCII,
                  const_cast<std::uint8_t*>(ascii));

    ctx.gtiff = GTIFNewSimpleTags(ctx.tiff);
    if (!ctx.gtiff)
        return nullptr;

    GTIFDefn defn;
    if (!GTIFGetDefn(ctx.gtiff, &defn))
        return nullptr;

    char* wkt = GTIFGetOGISDefn(ctx.gtiff, &defn);
    if (!wkt)
        return nullptr;

    char* out = duplicate(wkt);
    VSIFree(wkt);
    return out;
}

void pdal_native_geotiff_string_free(char* value)
{
    std::free(value);
}

}
